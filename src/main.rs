mod parser;
use bytes::Buf;
use bytes::BytesMut; // NEW
use std::io; // NEW
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// AsyncReadExt
// Provides methods for reading asynchronously.

// AsyncWriteExt
// Provides methods for writing asynchronously.

use tokio::net::{TcpListener, TcpStream};

// TcpListener
// Used to listen for incoming TCP connections.

// TcpStream
// Represents an individual connection between the server and one client.

// Client ───── TCP connection ───── Server
//                 │
//             TcpStream

//Normally Rust's main function cannot directly be async:
#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();
    //This initializes the logging system.
    //tracing_subscriber tells Rust where/how to display those logs

    let listner = TcpListener::bind("127.0.0.1:6379").await?;
    tracing::info!("listening on 127.0.0.1:6379");

    // You're telling the operating system:
    // "Create a TCP server and listen on port 6379."
    // 6379
    // This is the standard Redis port.

    loop {
        let (socket, addr) = listner.accept().await?;
        // socket → communication channel
        // addr   → client's address

        tracing::info!(%addr, "client connected");

        tokio::spawn(async move {
            //The server can continue accepting new clients while existing ones are being handled.
            if let Err(e) = handle(socket).await {
                tracing::error!(%e,"connection error");
            }
        });
    }
}

async fn handle(mut socket: TcpStream) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096); //This creates an array containing
                                                 // loop {
                                                 //     let n = socket.read(&mut buf).await?;
                                                 //     //Read data from the client and put it into buf.
                                                 //     if n == 0 {
                                                 //         return Ok(());
                                                 //     }
                                                 //     socket.write_all(b"+PONG\r\n").await?;
                                                 // }
    loop {
        // 1. Drain every complete command currently in the buffer
        while let Some(args) =
            parser::parse(&mut buf).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?
        {
            // 2. Dispatch on the command name
            match args[0].to_uppercase().as_str() {
                "PING" => socket.write_all(b"+PONG\r\n").await?,
                "ECHO" if args.len() > 1 => {
                    let reply = format!("${}\r\n{}\r\n", args[1].len(), args[1]);
                    socket.write_all(reply.as_bytes()).await?;
                }
                _ => socket.write_all(b"-ERR unknown command\r\n").await?,
            }
        }

        // 3. Need more bytes: read into buf (appends, doesn't overwrite)
        if socket.read_buf(&mut buf).await? == 0 {
            return Ok(()); // client closed
        }
    }
}
