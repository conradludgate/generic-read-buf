//! ReadBuf types and methods. See https://github.com/rust-lang/rust/issues/78485

use cl_generic_vec::{raw::Storage, ArrayVec, HeapVec, SimpleVec, SliceVec};
use std::{cmp, fmt, io, mem::MaybeUninit, ops::Deref};

/// A [`Storage`] of [`u8`]s
pub trait Bytes: Storage<Item = u8> {}
impl<S: Storage<Item = u8>> Bytes for S {}

/// A wrapper over [`io::Read`] to provide the custom read_buf* methods on stable
pub trait Read: io::Read {
    /// Pull some bytes from this source into the specified buffer.
    ///
    /// This is equivalent to the [`read`](io::Read::read) method, except that it is passed a [`ReadBufRef`] rather than `[u8]` to allow use
    /// with uninitialized buffers. The new data will be appended to any existing contents of `buf`.
    ///
    /// The default implementation delegates to `read`.
    fn read_buf(&mut self, buf: ReadBufRef<'_, impl Bytes>) -> io::Result<()> {
        default_read_buf(|b| self.read(b), buf)
    }

    /// Read the exact number of bytes required to fill `buf`.
    ///
    /// This is equivalent to the [`read_exact`](io::Read::read_exact) method, except that it is passed a [`ReadBufRef`] rather than `[u8]` to
    /// allow use with uninitialized buffers.
    fn read_buf_exact(&mut self, mut buf: ReadBufRef<'_, impl Bytes>) -> io::Result<()> {
        while buf.remaining() > 0 {
            let prev_filled = buf.filled().len();
            match Read::read_buf(self, buf.reborrow()) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }

            if buf.filled().len() == prev_filled {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "failed to fill buffer",
                ));
            }
        }

        Ok(())
    }
}

impl<R: io::Read> Read for R {}

pub(crate) fn default_read_buf<F>(read: F, mut buf: ReadBufRef<'_, impl Bytes>) -> io::Result<()>
where
    F: FnOnce(&mut [u8]) -> io::Result<usize>,
{
    let n = read(buf.initialize_unfilled())?;
    buf.add_filled(n);
    Ok(())
}

/// A wrapper around a byte buffer that is incrementally filled and initialized.
///
/// This type is a sort of "double cursor". It tracks three regions in the buffer: a region at the beginning of the
/// buffer that has been logically filled with data, a region that has been initialized at some point but not yet
/// logically filled, and a region at the end that is fully uninitialized. The filled region is guaranteed to be a
/// subset of the initialized region.
///
/// In summary, the contents of the buffer can be visualized as:
/// ```not_rust
/// [             capacity              ]
/// [ filled |         unfilled         ]
/// [    initialized    | uninitialized ]
/// ```
pub struct ReadBuf<S: Bytes> {
    filled: usize,
    buf: SimpleVec<S>,
}

impl<S: Bytes> fmt::Debug for ReadBuf<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadBuf")
            .field("init", &self.buf.len())
            .field("filled", &self.filled)
            .field("capacity", &self.buf.capacity())
            .finish()
    }
}

/// A [`ReadBuf`] that takes it's buffer from an existing slice
pub type ReadSlice<'a> = ReadBuf<&'a mut [MaybeUninit<u8>]>;
/// A [`ReadBuf`] that owns it's buffer using a [`Vec<u8>`]
pub type ReadVec = ReadBuf<Box<[MaybeUninit<u8>]>>;
/// A [`ReadBuf`] that owns it's buffer using a [`[MaybeUninit<u8>; N]`](array)
pub type ReadArray<const N: usize> = ReadBuf<[MaybeUninit<u8>; N]>;

impl<const N: usize> ReadArray<N> {
    /// Create a new uninitialised [`ReadBuf`] backed by an array
    /// Will begin with 0 filled bytes.
    pub fn new_uninit_array() -> Self {
        Self {
            filled: 0,
            buf: ArrayVec::new(),
        }
    }
}

/// Create a [`ReadBuf`] from a fully initialised array of bytes.
/// Will begin with 0 filled bytes.
impl<const N: usize> From<[u8; N]> for ReadArray<N> {
    fn from(buf: [u8; N]) -> Self {
        ReadBuf {
            filled: 0,
            buf: ArrayVec::from_array(buf),
        }
    }
}

/// Create a [`ReadBuf`] from a partially initialised vec of bytes.
/// Will begin with 0 filled bytes.
impl From<Vec<u8>> for ReadVec {
    fn from(buf: Vec<u8>) -> Self {
        ReadBuf {
            filled: 0,
            buf: buf.into(),
        }
    }
}

/// Create a [`ReadBuf`] from an uninitialised boxed-slice of bytes.
/// Will begin with 0 filled bytes.
impl From<Box<[MaybeUninit<u8>]>> for ReadVec {
    fn from(buf: Box<[MaybeUninit<u8>]>) -> Self {
        ReadBuf {
            filled: 0,
            buf: HeapVec::with_storage(buf),
        }
    }
}

/// Create a [`ReadBuf`] from an initialised slice of bytes.
/// Will begin with 0 filled bytes.
impl<'a> From<&'a mut [u8]> for ReadSlice<'a> {
    fn from(buf: &'a mut [u8]) -> Self {
        ReadBuf {
            filled: 0,
            buf: SliceVec::full(buf),
        }
    }
}

/// Create a [`ReadBuf`] from an uninitialised slice of bytes.
/// Will begin with 0 filled bytes.
impl<'a> From<&'a mut [MaybeUninit<u8>]> for ReadSlice<'a> {
    fn from(buf: &'a mut [MaybeUninit<u8>]) -> Self {
        ReadBuf {
            filled: 0,
            buf: unsafe { SliceVec::new(buf) },
        }
    }
}

impl<S: Bytes> ReadBuf<S> {
    /// Extract the bytes from the [`ReadBuf`]
    pub fn into_inner(self) -> SimpleVec<S> {
        assert_eq!(self.filled, self.buf.len());
        self.buf
    }

    /// Creates a new [`ReadBufRef`] referencing this `ReadBuf`.
    #[inline]
    pub fn borrow(&mut self) -> ReadBufRef<'_, S> {
        ReadBufRef { read_buf: self }
    }

    /// Returns the total capacity of the buffer.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Returns a shared reference to the filled portion of the buffer.
    #[inline]
    pub fn filled(&self) -> &[u8] {
        &self.buf[..self.filled]
    }

    /// Returns a mutable reference to the filled portion of the buffer.
    #[inline]
    pub fn filled_mut(&mut self) -> &mut [u8] {
        &mut self.buf[..self.filled]
    }

    /// Returns a shared reference to the initialized portion of the buffer.
    ///
    /// This includes the filled portion.
    #[inline]
    pub fn initialized(&self) -> &[u8] {
        self.buf.as_slice()
    }

    /// Returns a mutable reference to the initialized portion of the buffer.
    ///
    /// This includes the filled portion.
    #[inline]
    pub fn initialized_mut(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }

    /// Returns a mutable reference to the unfilled part of the buffer without ensuring that it has been fully
    /// initialized.
    ///
    /// # Safety
    ///
    /// The caller must not de-initialize portions of the buffer that have already been initialized.
    #[inline]
    pub unsafe fn unfilled_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        &mut self.buf.storage_mut().as_mut()[self.filled..]
    }

    /// Returns a mutable reference to the uninitialized part of the buffer.
    ///
    /// It is safe to uninitialize any of these bytes.
    #[inline]
    pub fn uninitialized_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.buf.spare_capacity_mut()
    }

    /// Returns a mutable reference to the unfilled part of the buffer, ensuring it is fully initialized.
    ///
    /// Since `ReadBuf` tracks the region of the buffer that has been initialized, this is effectively "free" after
    /// the first use.
    #[inline]
    pub fn initialize_unfilled(&mut self) -> &mut [u8] {
        // should optimize out the assertion
        self.initialize_unfilled_to(self.remaining())
    }

    /// Returns a mutable reference to the first `n` bytes of the unfilled part of the buffer, ensuring it is
    /// fully initialized.
    ///
    /// # Panics
    ///
    /// Panics if `self.remaining()` is less than `n`.
    #[inline]
    pub fn initialize_unfilled_to(&mut self, n: usize) -> &mut [u8] {
        assert!(self.remaining() >= n);

        let extra_init = self.buf.len() - self.filled;
        // If we don't have enough initialized, do zeroing
        if n > extra_init {
            let uninit = n - extra_init;
            let unfilled = &mut self.uninitialized_mut()[0..uninit];

            for byte in unfilled.iter_mut() {
                byte.write(0);
            }

            // SAFETY: we just initialized uninit bytes, and the previous bytes were already init
            unsafe {
                self.assume_init(n);
            }
        }

        let filled = self.filled;

        &mut self.initialized_mut()[filled..filled + n]
    }

    /// Returns the number of bytes at the end of the slice that have not yet been filled.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.capacity() - self.filled
    }

    /// Clears the buffer, resetting the filled region to empty.
    ///
    /// The number of initialized bytes is not changed, and the contents of the buffer are not modified.
    #[inline]
    pub fn clear(&mut self) {
        self.set_filled(0); // The assertion in `set_filled` is optimized out
    }

    /// Increases the size of the filled region of the buffer.
    ///
    /// The number of initialized bytes is not changed.
    ///
    /// # Panics
    ///
    /// Panics if the filled region of the buffer would become larger than the initialized region.
    #[inline]
    pub fn add_filled(&mut self, n: usize) {
        self.set_filled(self.filled + n);
    }

    /// Sets the size of the filled region of the buffer.
    ///
    /// The number of initialized bytes is not changed.
    ///
    /// Note that this can be used to *shrink* the filled region of the buffer in addition to growing it (for
    /// example, by a `Read` implementation that compresses data in-place).
    ///
    /// # Panics
    ///
    /// Panics if the filled region of the buffer would become larger than the initialized region.
    #[inline]
    pub fn set_filled(&mut self, n: usize) {
        assert!(n <= self.buf.len());

        self.filled = n;
    }

    /// Asserts that the first `n` unfilled bytes of the buffer are initialized.
    ///
    /// `ReadBuf` assumes that bytes are never de-initialized, so this method does nothing when called with fewer
    /// bytes than are already known to be initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the first `n` unfilled bytes of the buffer have already been initialized.
    #[inline]
    pub unsafe fn assume_init(&mut self, n: usize) {
        self.buf
            .set_len_unchecked(cmp::max(self.buf.len(), self.filled + n));
    }

    /// Appends data to the buffer, advancing the written position and possibly also the initialized position.
    ///
    /// # Panics
    ///
    /// Panics if `self.remaining()` is less than `buf.len()`.
    #[inline]
    pub fn append(&mut self, buf: &[u8]) {
        assert!(self.remaining() >= buf.len());

        // SAFETY: we do not de-initialize any of the elements of the slice
        unsafe {
            write_slice(&mut self.unfilled_mut()[..buf.len()], buf);
        }

        // SAFETY: We just added the entire contents of buf to the filled section.
        unsafe { self.assume_init(buf.len()) }
        self.add_filled(buf.len());
    }

    /// Returns the amount of bytes that have been filled.
    #[inline]
    pub fn filled_len(&self) -> usize {
        self.filled
    }

    /// Returns the amount of bytes that have been initialized.
    #[inline]
    pub fn initialized_len(&self) -> usize {
        self.buf.len()
    }
}

// from MaybeUninit::write_slice
unsafe fn write_slice<T>(this: &mut [MaybeUninit<T>], src: &[T])
where
    T: Copy,
{
    // SAFETY: &[T] and &[MaybeUninit<T>] have the same layout
    let uninit_src: &[MaybeUninit<T>] = core::mem::transmute(src);

    this.copy_from_slice(uninit_src);
}

/// A wrapper around [`&mut ReadBuf`](ReadBuf) which prevents the buffer that the [`ReadBuf`] points to from being replaced.
#[derive(Debug)]
pub struct ReadBufRef<'a, S: Bytes> {
    read_buf: &'a mut ReadBuf<S>,
}

impl<'a, S: Bytes> ReadBufRef<'a, S> {
    /// Creates a new `ReadBufRef` referencing the same `ReadBuf` as this one.
    pub fn reborrow(&mut self) -> ReadBufRef<'_, S> {
        ReadBufRef {
            read_buf: self.read_buf,
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer.
    #[inline]
    pub fn filled_mut(&mut self) -> &mut [u8] {
        self.read_buf.filled_mut()
    }

    /// Returns a mutable reference to the initialized portion of the buffer.
    ///
    /// This includes the filled portion.
    #[inline]
    pub fn initialized_mut(&mut self) -> &mut [u8] {
        self.read_buf.initialized_mut()
    }

    /// Returns a mutable reference to the unfilled part of the buffer without ensuring that it has been fully
    /// initialized.
    ///
    /// # Safety
    ///
    /// The caller must not de-initialize portions of the buffer that have already been initialized.
    #[inline]
    pub unsafe fn unfilled_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.read_buf.unfilled_mut()
    }

    /// Returns a mutable reference to the uninitialized part of the buffer.
    ///
    /// It is safe to uninitialize any of these bytes.
    #[inline]
    pub fn uninitialized_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.read_buf.uninitialized_mut()
    }

    /// Returns a mutable reference to the unfilled part of the buffer, ensuring it is fully initialized.
    ///
    /// Since `ReadBuf` tracks the region of the buffer that has been initialized, this is effectively "free" after
    /// the first use.
    #[inline]
    pub fn initialize_unfilled(&mut self) -> &mut [u8] {
        self.read_buf.initialize_unfilled()
    }

    /// Returns a mutable reference to the first `n` bytes of the unfilled part of the buffer, ensuring it is
    /// fully initialized.
    ///
    /// # Panics
    ///
    /// Panics if `self.remaining()` is less than `n`.
    #[inline]
    pub fn initialize_unfilled_to(&mut self, n: usize) -> &mut [u8] {
        self.read_buf.initialize_unfilled_to(n)
    }

    /// Clears the buffer, resetting the filled region to empty.
    ///
    /// The number of initialized bytes is not changed, and the contents of the buffer are not modified.
    #[inline]
    pub fn clear(&mut self) {
        self.read_buf.clear()
    }

    /// Increases the size of the filled region of the buffer.
    ///
    /// The number of initialized bytes is not changed.
    ///
    /// # Panics
    ///
    /// Panics if the filled region of the buffer would become larger than the initialized region.
    #[inline]
    pub fn add_filled(&mut self, n: usize) {
        self.read_buf.add_filled(n)
    }

    /// Sets the size of the filled region of the buffer.
    ///
    /// The number of initialized bytes is not changed.
    ///
    /// Note that this can be used to *shrink* the filled region of the buffer in addition to growing it (for
    /// example, by a `Read` implementation that compresses data in-place).
    ///
    /// # Panics
    ///
    /// Panics if the filled region of the buffer would become larger than the initialized region.
    #[inline]
    pub fn set_filled(&mut self, n: usize) {
        self.read_buf.set_filled(n)
    }

    /// Asserts that the first `n` unfilled bytes of the buffer are initialized.
    ///
    /// `ReadBuf` assumes that bytes are never de-initialized, so this method does nothing when called with fewer
    /// bytes than are already known to be initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the first `n` unfilled bytes of the buffer have already been initialized.
    #[inline]
    pub unsafe fn assume_init(&mut self, n: usize) {
        self.read_buf.assume_init(n)
    }

    /// Appends data to the buffer, advancing the written position and possibly also the initialized position.
    ///
    /// # Panics
    ///
    /// Panics if `self.remaining()` is less than `buf.len()`.
    #[inline]
    pub fn append(&mut self, buf: &[u8]) {
        self.read_buf.append(buf)
    }
}

impl<'a, S: Bytes> Deref for ReadBufRef<'a, S> {
    type Target = ReadBuf<S>;

    fn deref(&self) -> &ReadBuf<S> {
        &*self.read_buf
    }
}
