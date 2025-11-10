use std::{cell::UnsafeCell, ffi::CStr, io, ptr};

use ringbuf::{StaticRb, traits::SplitRef, wrap::caching::Caching};

const BUFFER_SIZE: usize = 1024;

pub type RingBuffer<T> = StaticRb<T, BUFFER_SIZE>;
pub type BufferProducer<'a, T> = Caching<&'a RingBuffer<T>, true, false>;
pub type BufferConsumer<'a, T> = Caching<&'a RingBuffer<T>, false, true>;

pub struct SharedRingBuffer<T: 'static> {
    buffer: UnsafeCell<&'static mut RingBuffer<T>>,
}

impl<T> SharedRingBuffer<T> {
    /// Creates shared memory and uses it as a ring buffer with 1024 elements.
    ///
    /// ## SAFETY
    ///
    /// This structure must use a unique name: you cannot create two buffers with the same name.
    pub unsafe fn new(name: &CStr) -> Self {
        let shmem_fd = unsafe { libc::shm_open(name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o644) };
        if shmem_fd < 0 {
            panic!("Failed to create/open shared memory: {}", io::Error::last_os_error());
        }

        let size = size_of::<RingBuffer<T>>();

        if unsafe { libc::ftruncate(shmem_fd, size as libc::off_t) } < 0 {
            unsafe { libc::close(shmem_fd) };
            panic!("Failed to set shared memory size: {}", io::Error::last_os_error());
        }

        let ptr = unsafe {
            libc::mmap(std::ptr::null_mut(), size, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED, shmem_fd, 0)
        };

        unsafe { libc::close(shmem_fd) };

        if ptr == libc::MAP_FAILED {
            panic!("Failed to mmap shared memory: {}", io::Error::last_os_error());
        }

        let ringbuf_ptr = ptr as *mut RingBuffer<T>;

        unsafe { ptr::write(ringbuf_ptr, RingBuffer::<T>::default()) };

        Self {
            buffer: UnsafeCell::new(unsafe { ringbuf_ptr.as_mut().unwrap_unchecked() }),
        }
    }

    /// Creates a producer and a consumer for logs.
    ///
    /// ## SAFETY
    ///
    /// This function should be called only once and not from multiple threads at the same time.
    pub unsafe fn split_ref(&'static self) -> (BufferProducer<'static, T>, BufferConsumer<'static, T>) {
        let buffer = unsafe { self.buffer.get().as_mut().unwrap_unchecked() };
        buffer.split_ref()
    }
}

unsafe impl<T> Sync for SharedRingBuffer<T> {}
