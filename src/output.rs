use std::sync::{Mutex, MutexGuard};
use std::fs::File;
use std::path::Path;
use std::io::{self, Stdout, StdoutLock, Write};
use std::ops::{Drop};

use error::DedupError;

/// Default buffer size is 8 MiB
const DEFAULT_BUF_SIZE: usize = 1 << 23;

pub enum LockableSink {
    Stdout(Stdout),
    File(Mutex<File>),
}

impl LockableSink {
    pub fn new<P: AsRef<Path>>(src: Option<P>) -> Result<Self, DedupError> {
        if let Some(ref p) = src {
            let f = File::create(p)?;
            Ok(LockableSink::File(Mutex::new(f)))
        } else {
            Ok(LockableSink::Stdout(io::stdout()))
        }
    }

    pub fn lock(&self) -> SinkLock {
        match self {
            &LockableSink::Stdout(ref out) => {
                SinkLock::Stdout(out.lock())
            }
            &LockableSink::File(ref mu_file) => {
                SinkLock::File(mu_file.lock().unwrap())
            }
        }
    }

    #[inline]
    pub fn lock_write(&self, buf: &[u8]) -> io::Result<usize> {
        match self {
            &LockableSink::Stdout(ref out) => {
                let mut lock = out.lock();
                let res = lock.write(buf)?;
                lock.flush()?;
                Ok(res)
            },
            &LockableSink::File(ref mu) => {
                let mut lock = mu.lock().unwrap();
                let res = lock.write(buf)?;
                lock.flush()?;
                Ok(res)
            }
        }
    }

    #[inline]
    pub fn lock_write_all(&self, buf: &[u8]) -> io::Result<()> {
        match self {
            &LockableSink::Stdout(ref out) => {
                let mut lock = out.lock();
                let res = lock.write_all(buf)?;
                lock.flush()?;
                Ok(res)
            },
            &LockableSink::File(ref file_mu) => {
                let mut lock = file_mu.lock().unwrap();
                let res = lock.write_all(buf)?;
                lock.flush()?;
                Ok(res)
            }
        }
    }
}

pub enum SinkLock<'a> {
    Stdout(StdoutLock<'a>),
    File(MutexGuard<'a, File>)
}
 
impl<'a> io::Write for SinkLock<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            SinkLock::Stdout(ref mut outlock) => outlock.write(buf),
            SinkLock::File(ref mut file_mu_guard) => file_mu_guard.write(buf),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            SinkLock::Stdout(ref mut outlock) => outlock.write_all(buf),
            SinkLock::File(ref mut file_mu_guard) => file_mu_guard.write_all(buf),
        }
    }

    fn flush (&mut self) -> io::Result<()> {
        match self {
            SinkLock::Stdout(ref mut outlock) => outlock.flush(),
            SinkLock::File(ref mut file_mu_guard) => file_mu_guard.flush(),
        }
    }
} 

pub struct TetheredBufWriter<'a> {
    buf: Vec<u8>,
    sink: &'a LockableSink,
    panicked: bool,
}

impl<'a> TetheredBufWriter<'a> {
    pub fn new(sink: &'a LockableSink) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, sink)
    }

    pub fn with_capacity(capacity: usize, sink: &'a LockableSink) -> Self {
        let buf = Vec::with_capacity(capacity);
        TetheredBufWriter{buf, sink, panicked: false}
    }

    #[inline]
    fn flush_buf(&mut self) -> io::Result<()> {
        match self.sink {
            &LockableSink::Stdout(ref out) => {
                let mut lock = out.lock();
                lock.write_all(&self.buf).and_then(|_| lock.flush())?
            },
            &LockableSink::File(ref mu) => {
                let mut lock = mu.lock().unwrap();
                lock.write_all(&self.buf).and_then(|_| lock.flush())?
            }
        }
        self.buf.clear();
        Ok(())
    }
}


impl<'a> io::Write for TetheredBufWriter<'a> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.buf.len() + buf.len() >= self.buf.capacity() {
            self.flush_buf()?;
        }

        if buf.len() >= self.buf.capacity() {
            self.panicked = true;
            let res = self.sink.lock_write(buf);
            self.panicked = false;
            res
        } else {
            self.buf.write(buf)
        }
    }

    #[inline]
    fn flush (&mut self) -> io::Result<()> {
        self.panicked = true;
        let ret = self.flush_buf();
        self.panicked = false;
        ret
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.panicked = true;
        let ret = self.sink.lock_write_all(buf);
        self.panicked = false;
        ret
    }
}

impl<'a> Drop for TetheredBufWriter<'a> {
    fn drop(&mut self) {
        if !self.panicked {
            let _ret = self.flush_buf();
        }
    }
}