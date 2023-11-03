// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::exit,
    sync::{Arc, Mutex},
    thread::{self},
};

use tauri::{async_runtime::Mutex as AsyncMutex, State};
struct AppState {
    pty_pairs: Arc<AsyncMutex<Vec<PtyPair>>>,
    writer: Arc<AsyncMutex<Box<dyn Write + Send>>>,
    reader: Arc<AsyncMutex<BufReader<Box<dyn Read + Send>>>>,
 }
/*struct AppState {
    pty_pair: Arc<AsyncMutex<PtyPair>>,
    writer: Arc<AsyncMutex<Box<dyn Write + Send>>>,
    reader: Arc<AsyncMutex<BufReader<Box<dyn Read + Send>>>>,
}*/

#[tauri::command]
async fn async_add_pty(state: State<'_, AppState>) -> Result<usize, ()> {
   let pty_system = native_pty_system();
   let pty_pair = pty_system
       .openpty(PtySize {
           rows: 24,
           cols: 80,
           pixel_width: 0,
           pixel_height: 0,
       })
       .unwrap();
   let index = state.pty_pairs.lock().await.len();
   state.pty_pairs.lock().await.push(pty_pair);
   println!("ptyIndex: {}", index);
   Ok(index)
}


#[tauri::command]
async fn async_create_shell(ptyIndex: usize, state: State<'_, AppState>) -> Result<(), String> {
    println!("PTY ready");
    #[cfg(target_os = "windows")]
    let mut cmd = CommandBuilder::new("powershell.exe");

    #[cfg(not(target_os = "windows"))]
    let mut cmd = CommandBuilder::new("bash");

    // add the $TERM env variable so we can use clear and other commands

    #[cfg(target_os = "windows")]
    cmd.env("TERM", "cygwin");

    #[cfg(not(target_os = "windows"))]
    cmd.env("TERM", "xterm-256color");

   println!("ptyIndex: {}", ptyIndex);



   let pty_pair = &state.pty_pairs.lock().await[ptyIndex];
   let child = pty_pair.slave.spawn_command(cmd).map_err(|err| err.to_string())?;

   let child = Arc::new(Mutex::new(child));

   thread::spawn({
       let child = Arc::clone(&child);
       move || {
           let status = child.lock().unwrap().wait().unwrap();
           exit(status.exit_code() as i32)
       }
   });

   Ok(())
}

// #[tauri::command]
// async fn async_write_to_pty(data: &str, state: State<'_, AppState>) -> Result<(), ()> {
//     write!(state.writer.lock().await, "{}", data).map_err(|_| ())
// }
#[tauri::command]
async fn async_write_to_pty(ptyIndex: usize, data: &str, state: State<'_, AppState>) -> Result<(), ()> {
    println!("Writing to PTY: {}", data);
    let pty_pair = &state.pty_pairs.lock().await[ptyIndex];
    let mut master = pty_pair.master.take_writer().unwrap();
    write!(master, "{}", data).map_err(|_| ())
}

#[tauri::command]
async fn async_read_from_pty(ptyIndex: usize, state: State<'_, AppState>) -> Result<Option<String>, ()> {
    let pty_pair = &state.pty_pairs.lock().await[ptyIndex];
    let mut reader = BufReader::new(pty_pair.master.try_clone_reader().unwrap());
    let data = {
        // Read all available text
        let data = reader.fill_buf().map_err(|_| ())?;

        // Send the data to the webview if necessary
        if data.len() > 0 {
            let text = std::str::from_utf8(data)
                .map(|v| Some(v.to_string()))
                .map_err(|_| ())?;
            println!("Read from PTY: {}", text.clone().unwrap());
            text
        } else {
            None
        }
    };

    if let Some(data) = &data {
        reader.consume(data.len());
    }

    Ok(data)
}

#[tauri::command]
async fn async_resize_pty(ptyIndex: usize, rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), ()> {
   let pty_pair = &state.pty_pairs.lock().await[ptyIndex];
   pty_pair
       .master
       .resize(PtySize {
           rows,
           cols,
           ..Default::default()
       })
       .map_err(|_| ())
}


fn main() {
    let pty_system = native_pty_system();

    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    let reader = pty_pair.master.try_clone_reader().unwrap();
    let writer = pty_pair.master.take_writer().unwrap();

    tauri::Builder::default()
    .manage(AppState {
        pty_pairs: Arc::new(AsyncMutex::new(vec![pty_pair])),
        writer: Arc::new(AsyncMutex::new(writer)),
        reader: Arc::new(AsyncMutex::new(BufReader::new(reader))),
    })
        .invoke_handler(tauri::generate_handler![
            async_write_to_pty,
            async_resize_pty,
            async_create_shell,
            async_read_from_pty,
            async_add_pty
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
