use cl_generic_read_buf::{Bytes, Read, ReadArray, ReadBuf};

use std::io::{self, Cursor};

fn read_buf_exact(mut buf: ReadBuf<impl Bytes>) {
    assert_eq!(buf.capacity(), 4);

    let mut c = Cursor::new(&b""[..]);
    assert_eq!(
        c.read_buf_exact(buf.borrow()).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );

    let mut c = Cursor::new(&b"123456789"[..]);
    c.read_buf_exact(buf.borrow()).unwrap();
    assert_eq!(buf.filled(), b"1234");

    buf.clear();

    c.read_buf_exact(buf.borrow()).unwrap();
    assert_eq!(buf.filled(), b"5678");

    buf.clear();

    assert_eq!(
        c.read_buf_exact(buf.borrow()).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn read_slice_exact() {
    let mut buf = [0; 4];
    read_buf_exact(ReadBuf::from(&mut buf[..]))
}

#[test]
fn read_vec_exact() {
    let buf = Vec::with_capacity(4);
    read_buf_exact(ReadBuf::from(buf))
}

#[test]
fn read_array_exact() {
    read_buf_exact(ReadArray::<4>::new_uninit_array())
}
