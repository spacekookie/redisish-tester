use std::io::{BufRead, BufReader, Read, Result, Write};
use std::net::TcpStream;

fn write<S: Into<String>>(stream: &mut TcpStream, elem: S) -> Result<()> {
    Ok(stream.write(elem.into().as_bytes()).map(|_| ())?)
}

fn store_element<S: Into<String>>(stream: &mut TcpStream, elem: S) -> Result<()> {
    write(stream, "PUB ".to_string() + &elem.into() + "\n")
}

fn get_element(stream: &mut TcpStream) -> Result<String> {
    write(stream, "GET\n")?;

    let reader = BufReader::new(stream);
    reader.lines().skip(1).take(1).collect()
}

use rayon::prelude::*;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

fn main() {
    let (send, recv) = mpsc::channel();
    let (dr_send, dr_recv) = mpsc::channel();

    let target = 100;

    let mut state: Vec<String> = Vec::with_capacity(target);
    let mut data_rate: Vec<Duration> = Vec::with_capacity(target * 2);

    // Thread that keeps track of state
    thread::spawn(move || {
        let mut last = Instant::now();
        while let Ok(s) = recv.recv() {
            state.push(s);

            // Only keep 500 elements, remove oldest
            if state.len() > 500 {
                state.remove(0);
            }

            let now = Instant::now();
            dr_send.send(now - last).expect("Failed to channel");
            last = now;
        }
    });

    // Thread that pretty-prints it
    thread::spawn(move || {
        let mut last = Instant::now();
        while let Ok(s) = dr_recv.recv() {
            data_rate.push(s);

            let now = Instant::now();
            if now - last > Duration::new(0, 500_000_000) {
                // let dr = data_rate.clone();
                println!("{:?}", s);
                    // "Average send-rate: {:?} msg/s",
                    // 1000 / (dr.par_iter().sum::<Duration>() / dr.len() as u32).as_millis()
                // );

                last = now;
            }
        }
    });

    let threads: Vec<JoinHandle<()>> = (0..4)
        .map(|i| {
            let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
            let send = send.clone();
            thread::spawn(move || loop {
                store_element(&mut stream, "Banana").expect("Failed to write!");
                send.send(get_element(&mut stream).expect("Failed to read!"))
                    .expect("...");
            })
        })
        .collect();

    threads.into_par_iter().for_each(|jh| jh.join().unwrap());
}
