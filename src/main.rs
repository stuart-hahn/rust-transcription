use clap::Parser;
use std::path::PathBuf;
use std::process::Command;
use std::io::Write;

#[derive(Parser)]
struct Args {
    /// Input MPEG-4 file
    input: PathBuf,
    /// Output Markdown file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let output = args.output.unwrap_or_else(|| {
        let mut p = args.input.clone();
        p.set_extension("md");
        p
    });

    let wav = tempfile::NamedTempFile::new()?;
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&args.input)
        .args(["-ac", "1", "-ar", "16000", "-vn"])
        .arg(wav.path())
        .status()?;
    if !status.success() {
        return Err("ffmpeg conversion failed".into());
    }

    let mut reader = hound::WavReader::open(wav.path())?;
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap_or(0)).collect();

    let ps_config = pocketsphinx::CmdLn::init(true, &[
        "pocketsphinx",
        "-hmm", "/usr/share/pocketsphinx/model/en-us/en-us",
        "-lm", "/usr/share/pocketsphinx/model/en-us/en-us.lm.bin",
        "-dict", "/usr/share/pocketsphinx/model/en-us/cmudict-en-us.dict",
    ])?;
    let ps = pocketsphinx::PsDecoder::init(ps_config);
    ps.start_utt(None)?;
    ps.process_raw(&samples, false, true)?;
    ps.end_utt()?;
    let text = ps.get_hyp().map(|t| t.0).unwrap_or_default();

    let mut file = std::fs::File::create(&output)?;
    writeln!(file, "# Transcription\n")?;
    for line in text.split('\n') {
        writeln!(file, "{}", line)?;
    }
    println!("Wrote {}", output.display());
    Ok(())
}
