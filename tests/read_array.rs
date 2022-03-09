use cl_generic_read_buf::ReadArray;

/// Test that ReadArray has the correct numbers when created from an initialised vec
#[test]
fn new() {
    let buf = [0; 16];
    let rbuf = ReadArray::from(buf);

    assert_eq!(rbuf.filled_len(), 0);
    assert_eq!(rbuf.initialized_len(), 16);
    assert_eq!(rbuf.capacity(), 16);
    assert_eq!(rbuf.remaining(), 16);
}

/// Test that ReadArray has the correct numbers when created from an uninitialised vec
#[test]
fn uninit() {
    let rbuf = ReadArray::<16>::new_uninit_array();

    assert_eq!(rbuf.filled_len(), 0);
    assert_eq!(rbuf.initialized_len(), 0);
    assert_eq!(rbuf.capacity(), 16);
    assert_eq!(rbuf.remaining(), 16);
}

#[test]
fn initialize_unfilled() {
    let mut rbuf = ReadArray::<16>::new_uninit_array();

    rbuf.initialize_unfilled();

    assert_eq!(rbuf.initialized_len(), 16);
}

#[test]
fn initialize_unfilled_to() {
    let mut rbuf = ReadArray::<16>::new_uninit_array();

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
    let buf = [0; 16];
    let mut rbuf = ReadArray::from(buf);

    rbuf.add_filled(1);

    assert_eq!(rbuf.filled_len(), 1);
    assert_eq!(rbuf.remaining(), 15);
}

#[test]
#[should_panic]
fn add_filled_panic() {
    let mut rbuf = ReadArray::<16>::new_uninit_array();

    rbuf.add_filled(1);
}

#[test]
fn set_filled() {
    let buf = [0; 16];
    let mut rbuf = ReadArray::from(buf);

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
    let mut rbuf = ReadArray::<16>::new_uninit_array();

    rbuf.set_filled(16);
}

#[test]
fn clear() {
    let buf = [255; 16];
    let mut rbuf = ReadArray::from(buf);

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
    let mut rbuf = ReadArray::<16>::new_uninit_array();

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
    let mut rbuf = ReadArray::<16>::new_uninit_array();

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
    let buf = [0; 16];
    let mut rbuf = ReadArray::from(buf);

    rbuf.add_filled(8);

    let filled = rbuf.filled().to_vec();

    assert_eq!(&*filled, &*rbuf.filled_mut());
}
