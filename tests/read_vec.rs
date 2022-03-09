use cl_generic_read_buf::ReadVec;

/// Test that ReadVec has the correct numbers when created from an initialised vec
#[test]
fn new() {
    let buf = vec![0; 16];
    let rbuf = ReadVec::from(buf);

    assert_eq!(rbuf.filled_len(), 0);
    assert_eq!(rbuf.initialized_len(), 16);
    assert_eq!(rbuf.capacity(), 16);
    assert_eq!(rbuf.remaining(), 16);
}

/// Test that ReadVec has the correct numbers when created from an uninitialised vec
#[test]
fn uninit() {
    let buf = Vec::with_capacity(16);
    let rbuf = ReadVec::from(buf);

    assert_eq!(rbuf.filled_len(), 0);
    assert_eq!(rbuf.initialized_len(), 0);
    assert_eq!(rbuf.capacity(), 16);
    assert_eq!(rbuf.remaining(), 16);
}

#[test]
fn initialize_unfilled() {
    let buf = Vec::with_capacity(16);
    let mut rbuf = ReadVec::from(buf);

    rbuf.initialize_unfilled();

    assert_eq!(rbuf.initialized_len(), 16);
}

#[test]
fn initialize_unfilled_to() {
    let buf = Vec::with_capacity(16);
    let mut rbuf = ReadVec::from(buf);

    rbuf.initialize_unfilled_to(8);

    assert_eq!(rbuf.initialized_len(), 8);

    rbuf.initialize_unfilled_to(4);

    assert_eq!(rbuf.initialized_len(), 8);

    rbuf.set_filled(8);

    rbuf.initialize_unfilled_to(6);

    assert_eq!(rbuf.initialized_len(), 14);

    rbuf.initialize_unfilled_to(8);

    assert_eq!(rbuf.initialized_len(), 16);
}

#[test]
fn add_filled() {
    let buf = vec![0; 16];
    let mut rbuf = ReadVec::from(buf);

    rbuf.add_filled(1);

    assert_eq!(rbuf.filled_len(), 1);
    assert_eq!(rbuf.remaining(), 15);
}

#[test]
#[should_panic]
fn add_filled_panic() {
    let buf = Vec::with_capacity(16);
    let mut rbuf = ReadVec::from(buf);

    rbuf.add_filled(1);
}

#[test]
fn set_filled() {
    let buf = vec![0; 16];
    let mut rbuf = ReadVec::from(buf);

    rbuf.set_filled(16);

    assert_eq!(rbuf.filled_len(), 16);
    assert_eq!(rbuf.remaining(), 0);

    rbuf.set_filled(6);

    assert_eq!(rbuf.filled_len(), 6);
    assert_eq!(rbuf.remaining(), 10);
}

#[test]
#[should_panic]
fn set_filled_panic() {
    let buf = Vec::with_capacity(16);
    let mut rbuf = ReadVec::from(buf);

    rbuf.set_filled(16);
}

#[test]
fn clear() {
    let buf = vec![255; 16];
    let mut rbuf = ReadVec::from(buf);

    rbuf.set_filled(16);

    assert_eq!(rbuf.filled_len(), 16);
    assert_eq!(rbuf.remaining(), 0);

    rbuf.clear();

    assert_eq!(rbuf.filled_len(), 0);
    assert_eq!(rbuf.remaining(), 16);

    assert_eq!(rbuf.initialized(), [255; 16]);
}

#[test]
fn assume_init() {
    let buf = Vec::with_capacity(16);
    let mut rbuf = ReadVec::from(buf);

    unsafe {
        rbuf.assume_init(8);
    }

    assert_eq!(rbuf.initialized_len(), 8);

    rbuf.add_filled(4);

    unsafe {
        rbuf.assume_init(2);
    }

    assert_eq!(rbuf.initialized_len(), 8);

    unsafe {
        rbuf.assume_init(8);
    }

    assert_eq!(rbuf.initialized_len(), 12);
}

#[test]
fn append() {
    let mut buf = vec![255; 16];
    buf.clear();
    let mut rbuf = ReadVec::from(buf);

    rbuf.append(&[0; 8]);

    assert_eq!(rbuf.initialized_len(), 8);
    assert_eq!(rbuf.filled_len(), 8);
    assert_eq!(rbuf.filled(), [0; 8]);

    rbuf.clear();

    rbuf.append(&[1; 16]);

    assert_eq!(rbuf.initialized_len(), 16);
    assert_eq!(rbuf.filled_len(), 16);
    assert_eq!(rbuf.filled(), [1; 16]);
}

#[test]
fn filled_mut() {
    let buf = vec![0; 16];
    let mut rbuf = ReadVec::from(buf);

    rbuf.add_filled(8);

    let filled = rbuf.filled().to_vec();

    assert_eq!(&*filled, &*rbuf.filled_mut());
}
