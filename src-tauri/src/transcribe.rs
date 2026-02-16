use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

pub struct Transcriber {
    state: WhisperState,
    #[allow(dead_code)]
    ctx: WhisperContext,
}

impl Transcriber {
    /// Load model with GPU, falling back to CPU on failure.
    pub fn new(model_path: &Path) -> Result<Self> {
        match Self::init(model_path, true) {
            Ok(t) => {
                log::info!("Whisper model loaded (GPU)");
                Ok(t)
            }
            Err(gpu_err) => {
                log::warn!("GPU init failed: {gpu_err:#}. Falling back to CPU.");
                Self::init(model_path, false)
                    .context("Failed to load model on both GPU and CPU")
            }
        }
    }

    fn init(model_path: &Path, use_gpu: bool) -> Result<Self> {
        let mut params = WhisperContextParameters::default();
        params.use_gpu(use_gpu);

        let ctx = WhisperContext::new_with_params(
            model_path
                .to_str()
                .context("Invalid model path encoding")?,
            params,
        )
        .map_err(|e| anyhow::anyhow!("Failed to load whisper model: {e}"))?;

        let state = ctx
            .create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {e}"))?;

        log::info!(
            "Whisper model loaded from {} (gpu={})",
            model_path.display(),
            use_gpu
        );
        Ok(Self { state, ctx })
    }

    pub fn transcribe(&mut self, audio: &[f32], language: &str) -> Result<String> {
        // Skip transcription if audio is silence (RMS energy below threshold)
        let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
        if rms < 0.001 {
            log::debug!("Audio too quiet (rms={rms:.4}), skipping transcription");
            return Ok(String::new());
        }

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
        params.set_language(Some(language));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_single_segment(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);

        self.state
            .full(params, audio)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {e}"))?;

        let n_segments = self
            .state
            .full_n_segments()
            .map_err(|e| anyhow::anyhow!("Failed to get segments: {e}"))?;

        let mut text = String::new();
        for i in 0..n_segments {
            if let Ok(segment) = self.state.full_get_segment_text(i) {
                text.push_str(&segment);
            }
        }

        Ok(text.trim().to_string())
    }
}

pub fn default_model_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("voice-to-text")
        .join("models")
}

pub fn default_model_path() -> PathBuf {
    default_model_dir().join("ggml-large-v3-turbo.bin")
}
