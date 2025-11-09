use std::ffi::CString;
use std::sync::atomic::{AtomicU32, Ordering};

use super::{LogEntry, RING_BUFFER_SIZE, RingBuffer};



pub(crate) unsafe fn init_shared_memory() -> *mut RingBuffer {
    let shm_name = CString::new(super::SHM_NAME).unwrap();
    
    // Создаем или открываем shared memory
    let fd = unsafe {
        libc::shm_open(
            shm_name.as_ptr(),
            libc::O_CREAT | libc::O_RDWR,
            0o644,
        )
    };
    
    if fd < 0 {
        panic!("Failed to create/open shared memory: {}", std::io::Error::last_os_error());
    }
    
    // Вычисляем размер RingBuffer
    let size = std::mem::size_of::<RingBuffer>();
    
    // Устанавливаем размер
    if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
        unsafe { libc::close(fd) };
        panic!("Failed to set shared memory size: {}", std::io::Error::last_os_error());
    }
    
    // Маппим в память
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    
    unsafe { libc::close(fd) };
    
    if ptr == libc::MAP_FAILED {
        panic!("Failed to mmap shared memory: {}", std::io::Error::last_os_error());
    }
    
    let ringbuf = ptr as *mut RingBuffer;
    
    // Инициализируем, если еще не инициализирован
    // Используем compare-and-swap на write_pos для атомарной инициализации
    // Пытаемся установить write_pos в u32::MAX как маркер инициализации
    let init_result = unsafe {
        (*ringbuf).write_pos.compare_exchange(0, u32::MAX, Ordering::AcqRel, Ordering::Acquire)
    };
    
    if init_result.is_ok() {
        // Мы первые, инициализируем структуру
        unsafe {
            std::ptr::write(ringbuf, RingBuffer {
                read_pos: AtomicU32::new(0),
                write_pos: AtomicU32::new(0),
                entries: std::mem::zeroed(),
            });
        }
    } else {
        // Кто-то другой инициализирует, ждем завершения
        // (write_pos будет установлен обратно в 0 после инициализации)
        while unsafe { (*ringbuf).write_pos.load(Ordering::Acquire) } == u32::MAX {
            std::hint::spin_loop();
        }
    }
    
    ringbuf
}

pub(crate) unsafe fn push(ringbuf: *mut RingBuffer, entry: &LogEntry) {
    let write_pos = unsafe { (*ringbuf).write_pos.fetch_add(1, Ordering::AcqRel) };
    let index = (write_pos as usize) % RING_BUFFER_SIZE;
    
    // Копируем entry в позицию index
    unsafe {
        std::ptr::write(
            &mut (*ringbuf).entries[index] as *mut LogEntry,
            *entry,
        );
    }
}

pub(crate) unsafe fn pop(ringbuf: *mut RingBuffer) -> Option<LogEntry> {
    let read_pos = unsafe { (*ringbuf).read_pos.load(Ordering::Acquire) };
    let write_pos = unsafe { (*ringbuf).write_pos.load(Ordering::Acquire) };
    
    if read_pos == write_pos {
        return None;
    }
    
    let index = (read_pos as usize) % RING_BUFFER_SIZE;
    let entry = unsafe { (*ringbuf).entries[index] };
    
    unsafe { (*ringbuf).read_pos.fetch_add(1, Ordering::AcqRel) };
    
    Some(entry)
}

pub(crate) unsafe fn is_empty(ringbuf: *mut RingBuffer) -> bool {
    let read_pos = unsafe { (*ringbuf).read_pos.load(Ordering::Acquire) };
    let write_pos = unsafe { (*ringbuf).write_pos.load(Ordering::Acquire) };
    read_pos == write_pos
}

