use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::net::{ToSocketAddrs, TcpStream};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::io::prelude::*;
use std::io::BufReader;
use std::thread::{self, JoinHandle};
use libc;

struct Server {
    hostname: String,
    port: u16,
    prefix: PathBuf,
    username: String
}

impl Server {
    pub fn new<P: AsRef<Path>>(hostname: &str, port: u16, username: &str, path: P) -> Result<Server, String> {
        let path = path.as_ref().join(hostname); 

        Self::create_pipe_pair(&path)
            .map_err(|e| format!("failed to create server pipes\n{}", e))?;

        Ok(Server {
            hostname: hostname.to_string(),
            port: port,
            prefix: path.to_owned(),
            username: username.to_string()
        })
    }

    pub fn run(&self) -> Result<(), String> {
        print!("connecting... ");
        let stream = TcpStream::connect((self.hostname.as_str(), self.port))
            .map_err(|e| format!("failed to connect to server\n{}", e))?;

        println!("done");

        let writer = stream.try_clone()
            .map_err(|e| format!("failed to clone stream\n{}", e))?;

        let child = self.run_writer(writer)?;

        let output = BufReader::new(stream);

        println!("starting listening");
        for line in output.lines() {
            println!("{:?}", line);
        }

        child.join().ok();

        Ok(())
    }

    fn run_writer(&self, mut stream: TcpStream) -> Result<JoinHandle<Result<(), String>>, String> {
        let file = self.prefix.join("out");

        let handle = thread::spawn(move || -> Result<(), String> {
            let pipe = File::open(file)
                .map_err(|e| format!("failed to open output pipe\n{}", e))?;
 
            let mut reader = BufReader::new(pipe);
            let mut buffer = String::new();

            loop {
                if let Ok(num_bytes) = reader.read_line(&mut buffer) {
                    if num_bytes > 0 {
                        println!("> {}", buffer);
                        stream.write_all(buffer.as_bytes())
                            .map_err(|e| format!("failed to write to stream\n{}", e))?;
                        stream.flush()
                            .map_err(|e| format!("failed to flush stream\n{}", e))?;
                        buffer.clear();
                    }
                }
            }
        });

        Ok(handle)
    }

    fn make_named_pipe<P: AsRef<Path>>(path: P) -> Result<(), String> {
        let path = CString::new(path.as_ref().as_os_str().as_bytes())
            .map_err(|_| "string contains NUL byte".to_string())?;

        unsafe {
            if libc::mkfifo(path.into_raw(), libc::S_IRWXU) != 0 {
                return Err(::std::io::Error::last_os_error().to_string())
            }
        }

        Ok(())
    }

    fn create_pipe_pair<P: AsRef<Path>>(path: P) -> Result<(), String> {
        let path = path.as_ref();

        fs::create_dir_all(path)
            .map_err(|e| format!("failed to create directory\n{}", e))?;

        Self::make_named_pipe(path.join("in"))
            .map_err(|e| format!("failed to create input pipe\n{}", e))?;

        Self::make_named_pipe(path.join("out"))
            .map_err(|e| format!("failed to create output pipe\n{}", e))?;

        Ok(())
    }
}

fn run() -> Result<(), String> {
    let server = Server::new("irc.rizon.net", 6660, "testnick", "/tmp/irc")
        .map_err(|e| format!("failed to create server\n{}", e))?;

    server.run()
}

fn main() {
    println!("{:?}", run());
}
