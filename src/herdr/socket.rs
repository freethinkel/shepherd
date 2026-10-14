//! herdr's sockets, by path: a unix socket there, or on Windows the named pipe herdr names after the path (its
//! `ipc.rs` makes the path string a namespaced local socket name and leaves a marker file at the path).
#[cfg(unix)]
use std::{io, path::Path};

#[cfg(unix)]
pub use std::os::unix::net::UnixStream as Stream;
#[cfg(unix)]
pub use tokio::net::UnixStream as AsyncStream;

#[cfg(unix)]
pub fn connect(path: &Path) -> io::Result<Stream> {
    Stream::connect(path)
}

#[cfg(unix)]
pub async fn connect_async(path: &Path) -> io::Result<AsyncStream> {
    AsyncStream::connect(path).await
}

/// Accepts connections at `path` (made anew), each handed to `serve` on `rt`.
#[cfg(unix)]
pub fn listen<F: Future<Output = ()> + Send + 'static>(path: &Path, rt: &tokio::runtime::Handle, serve: impl Fn(AsyncStream) -> F + Send + 'static) -> io::Result<()> {
    let _ = std::fs::remove_file(path); // left by an earlier Shepherd
    let listener = {
        let _guard = rt.enter();
        tokio::net::UnixListener::bind(path)?
    };
    rt.spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve(stream));
        }
    });
    Ok(())
}

#[cfg(windows)]
pub use pipe::{AsyncStream, Stream, connect, connect_async, listen};

#[cfg(windows)]
mod pipe {
    use interprocess::local_socket::traits::tokio::{Listener as _, Stream as _};
    use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Name, ToNsName, tokio, traits::Stream as _};
    use std::io::{self, Read, Write};
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    pub type AsyncStream = tokio::Stream;

    fn name(path: &Path) -> io::Result<Name<'static>> {
        path.to_string_lossy().into_owned().to_ns_name::<GenericNamespaced>()
    }

    pub fn connect(path: &Path) -> io::Result<Stream> {
        interprocess::local_socket::Stream::connect(name(path)?).map(|s| Stream(Arc::new(s)))
    }

    pub async fn connect_async(path: &Path) -> io::Result<AsyncStream> {
        AsyncStream::connect(name(path)?).await
    }

    pub fn listen<F: Future<Output = ()> + Send + 'static>(path: &Path, rt: &::tokio::runtime::Handle, serve: impl Fn(AsyncStream) -> F + Send + 'static) -> io::Result<()> {
        let listener = {
            let _guard = rt.enter();
            ListenerOptions::new().name(name(path)?).create_tokio()?
        };
        std::fs::write(path, b"")?; // herdr's marker: something listens here
        rt.spawn(async move {
            while let Ok(stream) = listener.accept().await {
                ::tokio::spawn(serve(stream));
            }
        });
        Ok(())
    }

    /// A named pipe that clones like a unix socket: one handle, shared. Its reads and writes are overlapped, so a
    /// reader thread does not block a writer.
    #[derive(Debug)]
    pub struct Stream(Arc<interprocess::local_socket::Stream>);

    impl Stream {
        pub fn try_clone(&self) -> io::Result<Self> {
            Ok(Self(self.0.clone()))
        }

        // ponytail: named pipes have no timeouts, so a herdr that never answers a hello holds the connecting thread;
        // a watchdog that drops the pipe if that ever shows up
        pub fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
            Ok(())
        }

        /// herdr hangs up after a `Detach`, which ends the reader; the handle closes with the last clone.
        pub fn shutdown(&self, _: std::net::Shutdown) -> io::Result<()> {
            Ok(())
        }
    }

    impl Read for Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            (&*self.0).read(buf)
        }
    }

    impl Write for Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            (&*self.0).write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Write for &Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            (&*self.0).write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
