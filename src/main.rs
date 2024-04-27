use clap::Parser;
use eyre::eyre;
use std::path::PathBuf;
use std::process::Command;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use tempdir::TempDir;
use wry::WebViewBuilder;

use fs_extra::{dir, copy_items};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to Asciidoctor document
    #[arg()]
    path: PathBuf,
}

fn main() -> eyre::Result<()> {
    let args = Args::try_parse()?;

    if !args.path.is_file() {
        return Err(eyre!("Expect Asciidoctor document as parameter!"));
    }

    let source_dir = args.path.parent().unwrap();

    let tmp_dir = TempDir::new("asciidoctor-preview")?;

    let options = dir::CopyOptions::new(); //Initialize default values for CopyOptions

    // copy dir1 and file1.txt to target/dir1 and target/file1.txt
    let mut from_paths = Vec::new();

    for entry in source_dir.read_dir()? {
        if let Ok(entry) = entry {
            from_paths.push(entry.path());
        }
    }

    copy_items(&from_paths, &tmp_dir, &options)?;

    let html_file = tmp_dir.path().join("index.html");
    let html_file = html_file.to_str().unwrap();
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let html_file = str::replace(html_file, "\\", "/");

    let output = Command::new("cmd")
        .args([
            "/C",
            "asciidoctor",
            "-a",
            "toc=left",
            "-a",
            "source-highlighter=highlight.js",
            "-o",
            "index.html",
            "-R",
            source_dir.to_str().unwrap(),
            "-D",
            tmp_dir.path().to_str().unwrap(),
            args.path.to_str().unwrap(),
        ])
        .output()?;

    let conversion_error = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(eyre!(
            "Can't convert document with error - {}",
            conversion_error
        ));
    }
    let mut tmp_dir = Option::Some(tmp_dir);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Asciidoctor Preview")
        .with_maximized(true)
        .build(&event_loop)?;

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let builder = WebViewBuilder::new(&window);

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let builder = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        WebViewBuilder::new_gtk(vbox)
    };

    let _webview = builder.with_url(&html_file).build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            tmp_dir.take().map(|temp_dir| temp_dir.close());
            *control_flow = ControlFlow::Exit
        }
    });
}
