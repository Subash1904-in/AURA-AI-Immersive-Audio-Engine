#[cfg(test)]
mod tests {
    use arc_swap::ArcSwap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use crate::audio::analysis::AnalysisEngine;
    use crate::audio::dsp::params::DspParams;

    #[test]
    fn test_visualizer_event_throttling_and_disabling() {
        let params_bus = Arc::new(ArcSwap::from_pointee(DspParams::default()));

        // 1. Enable visualizer
        {
            let mut params = params_bus.load_full().as_ref().clone();
            params.visualizer_enabled = true;
            params_bus.store(Arc::new(params));
        }

        let engine = Arc::new(AnalysisEngine::new(44100.0, params_bus.clone()));

        // Count received events
        let event_count = Arc::new(AtomicUsize::new(0));
        let event_count_clone = event_count.clone();

        engine.set_visualizer_callback(Some(Arc::new(move |_payload| {
            event_count_clone.fetch_add(1, Ordering::SeqCst);
        })));

        // Feed blocks of samples to trigger processing
        // Feed blocks repeatedly over ~200ms to allow throttled emissions.
        // Cap is 25ms, so in 200ms we expect at most ~8 events.
        let samples = vec![0.0f32; 2048];
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(200) {
            engine.push_samples(&samples);
            thread::sleep(Duration::from_millis(10));
        }

        // Wait a tiny bit for the worker thread to finish processing
        thread::sleep(Duration::from_millis(50));

        let count_enabled = event_count.load(Ordering::SeqCst);
        assert!(
            count_enabled > 0,
            "Expected at least one visualizer event when enabled"
        );
        assert!(
            count_enabled <= 10,
            "Expected throttled events (got {} events, expected <= 10)",
            count_enabled
        );

        // 2. Disable visualizer
        {
            let mut params = params_bus.load_full().as_ref().clone();
            params.visualizer_enabled = false;
            params_bus.store(Arc::new(params));
        }

        // Reset count
        event_count.store(0, Ordering::SeqCst);

        // Push samples for another 100ms
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(100) {
            engine.push_samples(&samples);
            thread::sleep(Duration::from_millis(10));
        }

        thread::sleep(Duration::from_millis(50));

        let count_disabled = event_count.load(Ordering::SeqCst);
        assert_eq!(
            count_disabled, 0,
            "Expected exactly zero visualizer events when disabled"
        );
    }
}
