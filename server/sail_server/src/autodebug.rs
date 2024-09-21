use std::env;
use std::io;
// TODO: Can't use this in WASI.
// use std::os::unix::net::UnixStream;
use std::process;

fn vscode_debug_send_pid() -> io::Result<()> {
    // If debugging is enabled. This is so you can just always run this.
    if env::var("CPP_DEBUG").as_deref() != Ok("1") {
        return Ok(());
    }

    // Get the Unix socket path to connect to.
    let handle = env::var("AUTODEBUG_IPC_HANDLE").map_err(|_| {
        eprintln!("autodebug: socket path not set; there should be a AUTODEBUG_IPC_HANDLE env var - try restarting the terminal");
        io::Error::new(io::ErrorKind::NotFound, "AUTODEBUG_IPC_HANDLE not set")
    })?;

    eprintln!("autodebug: starting debug session");

    // // Create a Unix socket.
    // let stream = UnixStream::connect(&handle)?;

    // let message = format!(
    //     "{{ \"type\": \"lldb\", \"request\": \"attach\", \"pid\": {} }}\n",
    //     process::id()
    // );

    // // Write the message to the socket.
    // stream.write_all(message)?;

    // stream.close()?;

    Ok(())
}
