// SpeechHelper.swift
// Swift helper library for macOS Speech Recognition
// Provides C-compatible FFI for use from Rust
// Supports both Classic (SFSpeechRecognizer) and Tahoe (macOS 15.0 Tahoe SpeechTranscriber) modes

import Foundation
import Speech
import AVFoundation

// MARK: - Global Callbacks

private var resultCallback: (@convention(c) (UnsafePointer<CChar>?, Int32) -> Void)?
private var errorCallback: (@convention(c) (UnsafePointer<CChar>?) -> Void)?
private var readyCallback: (@convention(c) () -> Void)?
private var hasNotifiedReady = false
private var speechTimeout: TimeInterval = 30.0 // Default fallback, updated by init

// MARK: - Tahoe Manager (macOS 15.0 Tahoe)

@available(macOS 26.0, *)
final class TahoeSession {
    private var engine: AVAudioEngine?
    private var analyzer: SpeechAnalyzer?
    private var transcriber: DictationTranscriber?
    private var mainTask: Task<Void, Never>?
    private var continuation: AsyncStream<AnalyzerInput>.Continuation?
    private var isRunning = false
    private var bufferCount = 0
    private var accumulatedText = ""
    private var converter: AVAudioConverter?
    
    func start(locale: Locale) {
        guard !isRunning else { return }
        isRunning = true
        bufferCount = 0
        accumulatedText = ""
        
        print("[Tahoe] Session starting for \(locale.identifier)...")
        
        mainTask = Task {
            do {
                // 1. Setup Locale
                guard let supportedLocale = await DictationTranscriber.supportedLocale(equivalentTo: locale) else {
                    print("[Tahoe] ERROR: Locale not supported")
                    return
                }
                
                // 2. Transcriber (Stage 5: Finalized Implementation)
                // Use explicit options to ensure real-time volatile results and frequent synchronization
                let transcriber = DictationTranscriber(
                    locale: supportedLocale,
                    contentHints: [],
                    transcriptionOptions: [.punctuation, .emoji],
                    reportingOptions: [.volatileResults, .frequentFinalization],
                    attributeOptions: [.transcriptionConfidence]
                )
                self.transcriber = transcriber
                
                // 3. Assets
                if let req = try await AssetInventory.assetInstallationRequest(supporting: [transcriber]) {
                    if req.progress.fractionCompleted < 1.0 {
                        print("[Tahoe] Downloading models...")
                        try await req.downloadAndInstall()
                    }
                }
                print("[Tahoe] Models ready")
                
                // 4. Analyzer
                let analyzer = SpeechAnalyzer(modules: [transcriber])
                self.analyzer = analyzer
                
                // 5. Audio engine setup
                let engine = AVAudioEngine()
                self.engine = engine
                let inputNode = engine.inputNode
                let rawFormat = inputNode.outputFormat(forBus: 0)
                print("[Tahoe] Raw input format: \(rawFormat.sampleRate)Hz \(rawFormat.channelCount)ch")
                
                // 5.1 Early Format Negotiation (RESEARCH04)
                print("[Tahoe] [TRC] Negotiating best available audio format...")
                let suggestedFormat = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber])
                let bestFormat = suggestedFormat ?? rawFormat
                print("[Tahoe] [DBG] Selected format for recognition: \(bestFormat.sampleRate)Hz \(bestFormat.channelCount)ch")
                if suggestedFormat == nil {
                    print("[Tahoe] [WRN] No suggested format from analyzer, using raw format.")
                }
                
                // 6. Robust Stream Creation
                let (stream, continuation) = AsyncStream<AnalyzerInput>.makeStream()
                self.continuation = continuation
                
                // 7. Install Tap with Manual Resampling (RESEARCH04/Stage 3)
                inputNode.removeTap(onBus: 0)
                
                // If format is different, we need a converter
                if bestFormat.sampleRate != rawFormat.sampleRate || bestFormat.channelCount != rawFormat.channelCount {
                    print("[Tahoe] [TRC] Setting up manual converter from \(rawFormat.sampleRate)Hz to \(bestFormat.sampleRate)Hz")
                    self.converter = AVAudioConverter(from: rawFormat, to: bestFormat)
                }
                
                print("[Tahoe] [TRC] Installing Tap with raw format: \(rawFormat.sampleRate)Hz")
                inputNode.installTap(onBus: 0, bufferSize: 1024, format: rawFormat) { [weak self] buffer, _ in
                    guard let self = self else { return }
                    self.bufferCount += 1
                    
                    var bufferToYield = buffer
                    
                    // Apply conversion if necessary
                    if let converter = self.converter {
                        let ratio = bestFormat.sampleRate / rawFormat.sampleRate
                        let targetCapacity = AVAudioFrameCount(Double(buffer.frameLength) * ratio) + 1
                        if let convertedBuffer = AVAudioPCMBuffer(pcmFormat: bestFormat, frameCapacity: targetCapacity) {
                            var error: NSError?
                            let status = converter.convert(to: convertedBuffer, error: &error) { inNumPackets, outStatus in
                                outStatus.pointee = .haveData
                                return buffer
                            }
                            if status != .error && error == nil {
                                bufferToYield = convertedBuffer
                            } else if let err = error {
                                if self.bufferCount % 100 == 0 {
                                    print("[Tahoe] [ERR] Conversion error: \(err.localizedDescription)")
                                }
                            }
                        }
                    }
                    
                    if self.bufferCount <= 5 || self.bufferCount % 100 == 0 {
                        print("[Tahoe] Buffer #\(self.bufferCount) (Yielding: \(bufferToYield.format.sampleRate)Hz)")
                    }
                    
                    if let copy = self.copyBuffer(bufferToYield) {
                        continuation.yield(AnalyzerInput(buffer: copy))
                    }
                }
                
                // 8. Start Engine
                engine.prepare()
                try engine.start()
                print("[Tahoe] Engine running")
                
                // 9. IMPORTANT: Wait for buffers to flow before starting analyzer
                print("[Tahoe] Waiting for initial buffers...")
                while self.bufferCount < 5 && !Task.isCancelled {
                    try? await Task.sleep(nanoseconds: 20_000_000) // 20ms
                }
                print("[Tahoe] Audio flowing, proceeding to analyzer")
                
                // Notify Ready (Symmetry with Classic/Capture)
                await MainActor.run { readyCallback?() }
                
                // 10. Run Analysis and Results loop in a TaskGroup (Stage 2: Restore & Resample)
                print("[Tahoe] [TRC] Entering TaskGroup with bestFormat: \(bestFormat.sampleRate)Hz")
                await withTaskGroup(of: Void.self) { group in
                    // Sub-task: Analyzer Process
                    group.addTask {
                        print("[Tahoe] [TRC] Analyzer task started.")
                        do {
                            print("[Tahoe] [TRC] Calling analyzer.prepareToAnalyze(in: bestFormat)...")
                            try await analyzer.prepareToAnalyze(in: bestFormat)
                            print("[Tahoe] [TRC] prepareToAnalyze completed successfully.")
                            
                            print("[Tahoe] [TRC] Calling analyzer.start(inputSequence: stream)...")
                            try await analyzer.start(inputSequence: stream)
                            print("[Tahoe] [TRC] analyzer.start() returned normally, engine active.")
                            
                            print("[Tahoe] [TRC] Entering engine heartbeat loop...")
                            var heartbeatCount = 0
                            while self.isRunning && !Task.isCancelled {
                                try? await Task.sleep(nanoseconds: 500_000_000) // 500ms
                                heartbeatCount += 1
                                if heartbeatCount % 10 == 0 {
                                    print("[Tahoe] [TRC] Heartbeat: session alive, buffers=\(self.bufferCount)")
                                }
                            }
                            
                            await analyzer.cancelAndFinishNow()
                        } catch {
                            print("[Tahoe] [ERR] Analyzer Task Catch: \(error)")
                        }
                    }
                    
                    // Sub-task: Results Collection (Restored)
                    group.addTask {
                        print("[Tahoe] [TRC] Results task started.")
                        do {
                            print("[Tahoe] [TRC] TRACE 2: Mandatory 500ms warm-up delay...")
                            try? await Task.sleep(nanoseconds: 500_000_000)
                            
                            print("[Tahoe] [TRC] TRACE 3: Accessing transcriber.results properties...")
                            for try await result in transcriber.results {
                                if Task.isCancelled || !self.isRunning { break }
                                
                                let rawSegmentText = String(result.text.characters)
                                let isFinal = result.isFinal
                                
                                // Deduplicate overlap with accumulatedText (Stage 5 specialized logic)
                                var cleanSegment = rawSegmentText
                                let maxOverlap = min(self.accumulatedText.count, rawSegmentText.count)
                                if maxOverlap > 0 {
                                    for i in stride(from: maxOverlap, through: 1, by: -1) {
                                        if self.accumulatedText.suffix(i) == rawSegmentText.prefix(i) {
                                            cleanSegment = String(rawSegmentText.dropFirst(i))
                                            break
                                        }
                                    }
                                }
                                
                                // Cumulative text for Rust's input_diff
                                let fullText = self.accumulatedText + cleanSegment
                                
                                if rawSegmentText.isEmpty && !isFinal { continue }
                                
                                print("[Tahoe] [TRC] Result: '\(rawSegmentText)' -> clean: '\(cleanSegment)' (isFinal=\(isFinal ? 1 : 0))")
                                
                                await MainActor.run {
                                    if let cb = resultCallback {
                                        fullText.withCString { ptr in
                                            cb(ptr, isFinal ? 1 : 0)
                                        }
                                    }
                                }
                                
                                if isFinal {
                                    self.accumulatedText += cleanSegment
                                }
                            }
                            print("[Tahoe] [TRC] Results loop exited normally.")
                        } catch {
                            print("[Tahoe] [ERR] Results Task Catch: \(error)")
                        }
                    }
                }
                
                print("[Tahoe] [TRC] Session task group finished.")
                
                // Notify Rust about completion (Symmetry with Windows OnSessionCompleted)
                "COMPLETED:TahoeSessionEnded".withCString { errorCallback?($0) }
                
            } catch {
                print("[Tahoe] Fatal session error: \(error)")
                "\(error)".withCString { errorCallback?($0) }
            }
            print("[Tahoe] Session task ending")
        }
    }
    
    func stop() {
        guard isRunning else { return }
        isRunning = false
        print("[Tahoe] Session stopping...")
        
        continuation?.finish()
        continuation = nil
        
        mainTask?.cancel()
        mainTask = nil
        
        engine?.stop()
        engine?.inputNode.removeTap(onBus: 0)
        engine = nil
        
        analyzer = nil
        transcriber = nil
        converter = nil
        accumulatedText = ""
        print("[Tahoe] Session stopped")
    }
    
    private func copyBuffer(_ buffer: AVAudioPCMBuffer) -> AVAudioPCMBuffer? {
        guard let copy = AVAudioPCMBuffer(pcmFormat: buffer.format, frameCapacity: buffer.frameCapacity) else {
            return nil
        }
        copy.frameLength = buffer.frameLength
        
        let chCount = Int(buffer.format.channelCount)
        let frameLen = Int(buffer.frameLength)
        
        if let src = buffer.floatChannelData, let dst = copy.floatChannelData {
            for ch in 0..<chCount {
                memcpy(dst[ch], src[ch], frameLen * MemoryLayout<Float>.size)
            }
        } else if let src = buffer.int16ChannelData, let dst = copy.int16ChannelData {
            for ch in 0..<chCount {
                memcpy(dst[ch], src[ch], frameLen * MemoryLayout<Int16>.size)
            }
        }
        return copy
    }
}

@available(macOS 26.0, *)
private var tahoeSession: TahoeSession?

// MARK: - Classic Mode (SFSpeechRecognizer)

private var classicEngine: AVAudioEngine?
private var classicRequest: SFSpeechAudioBufferRecognitionRequest?
private var classicTask: SFSpeechRecognitionTask?
private var classicRecognizer: SFSpeechRecognizer?
private var classicTimer: Timer? // For 30s timeout symmetry

private let kSuccess: Int32 = 0
private let kErrOsVersion: Int32 = -10

@_cdecl("speech_helper_init")
public func speechHelperInit(timeout: Double) -> Int32 {
    speechTimeout = timeout
    
    if #available(macOS 26.0, *) {
        print("[SpeechHelper] Initialized (macOS 15.0 Tahoe Mode Available via SDK 26.0, Timeout: \(timeout)s)")
    } else {
        print("[SpeechHelper] Initialized (Classic Mode Only, Timeout: \(timeout)s)")
    }
    return 0
}

@_cdecl("speech_helper_request_authorization")
public func speechHelperRequestAuthorization() -> Int32 {
    SFSpeechRecognizer.requestAuthorization { _ in }
    return 0
}

@_cdecl("speech_helper_set_result_callback")
public func speechHelperSetResultCallback(_ cb: @escaping @convention(c) (UnsafePointer<CChar>?, Int32) -> Void) {
    resultCallback = cb
}

@_cdecl("speech_helper_set_error_callback")
public func speechHelperSetErrorCallback(_ cb: @escaping @convention(c) (UnsafePointer<CChar>?) -> Void) {
    errorCallback = cb
}

@_cdecl("speech_helper_set_ready_callback")
public func speechHelperSetReadyCallback(_ cb: @escaping @convention(c) () -> Void) {
    readyCallback = cb
}

@_cdecl("speech_helper_start")
public func speechHelperStart(_ localePtr: UnsafePointer<CChar>?) -> Int32 {
    let localeStr = localePtr != nil ? String(cString: localePtr!) : "ja-JP"
    let locale = Locale(identifier: localeStr)
    
    hasNotifiedReady = false // Reset ready flag
    
    guard let recognizer = SFSpeechRecognizer(locale: locale), recognizer.isAvailable else { return -1 }
    classicRecognizer = recognizer

    let engine = AVAudioEngine()
    classicEngine = engine
    let req = SFSpeechAudioBufferRecognitionRequest()
    classicRequest = req
    if #available(macOS 13, *) { req.addsPunctuation = true }
    req.shouldReportPartialResults = true
    
    let inputNode = engine.inputNode
    let format = inputNode.outputFormat(forBus: 0)
    inputNode.removeTap(onBus: 0)
    inputNode.installTap(onBus: 0, bufferSize: 1024, format: format) { buffer, _ in
        if !hasNotifiedReady {
            hasNotifiedReady = true
            DispatchQueue.main.async { readyCallback?() }
        }
        req.append(buffer)
    }

    classicTask = recognizer.recognitionTask(with: req) { result, _ in
            guard let result = result else { return }
            let text = result.bestTranscription.formattedString
            let isFinal: Int32 = result.isFinal ? 1 : 0
            text.withCString { resultCallback?($0, isFinal) }
            
            // --- Timeout Symmetry (Classic) ---
            // Reset the 30s timer whenever we get a result.
            // This mimics Windows InitialSilenceTimeout/EndSilenceTimeout behavior logic manually.
            DispatchQueue.main.async {
                classicTimer?.invalidate()
                classicTimer = Timer.scheduledTimer(withTimeInterval: speechTimeout, repeats: false) { _ in
                    print("[Classic] Silence Timeout - Auto Stopping for Symmetry")
                    speechHelperStop()
                    "COMPLETED:ClassicTimeout".withCString { errorCallback?($0) }
                }
            }
        }

    // Determine initial compatibility
    if #available(macOS 26.0, *) {
         // Tahoe mode handles timeouts internally, but Classic mode needs this timer.
    }

    // Start initial timer (for initial silence)
    DispatchQueue.main.async {
        classicTimer?.invalidate()
        classicTimer = Timer.scheduledTimer(withTimeInterval: speechTimeout, repeats: false) { _ in
             print("[Classic] Initial Silence Timeout")
             speechHelperStop()
             "COMPLETED:ClassicInitialTimeout".withCString { errorCallback?($0) }
        }
    }

    engine.prepare()
    do { try engine.start() } catch { return -4 }
    print("[Classic] Started")
    return 0
}

@_cdecl("speech_helper_stop")
public func speechHelperStop() {
    classicTimer?.invalidate()
    classicTimer = nil
    
    classicEngine?.stop()
    classicEngine?.inputNode.removeTap(onBus: 0)
    classicRequest?.endAudio()
    classicTask?.cancel()
    classicRequest = nil
    classicTask = nil
    classicEngine = nil
    classicRecognizer = nil
}

@_cdecl("speech_helper_cleanup")
public func speechHelperCleanup() {
    speechHelperStop()
    if #available(macOS 26.0, *) { tahoeSession?.stop(); tahoeSession = nil }
    resultCallback = nil
    errorCallback = nil
}

@_cdecl("speech_helper_tick")
public func speechHelperTick() {
    RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.01))
}

// MARK: - Native Capture (OpenAI Mode)

private var captureEngine: AVAudioEngine?
private var audioDataCallback: (@convention(c) (UnsafePointer<Float>?, Int32, Int32) -> Void)?

@_cdecl("speech_helper_set_audio_data_callback")
public func speechHelperSetAudioDataCallback(_ cb: (@convention(c) (UnsafePointer<Float>?, Int32, Int32) -> Void)?) {
    audioDataCallback = cb
}

@_cdecl("speech_helper_start_capture")
public func speechHelperStartCapture() -> Int32 {
    let engine = AVAudioEngine()
    captureEngine = engine
    let inputNode = engine.inputNode
    let format = inputNode.outputFormat(forBus: 0)
    
    inputNode.removeTap(onBus: 0)
    inputNode.installTap(onBus: 0, bufferSize: 1024, format: format) { buffer, _ in
        guard let cb = audioDataCallback else { return }
        let count = Int32(buffer.frameLength)
        let rate = Int32(format.sampleRate)
        
        // Ensure we send Float data (usually standard on macOS)
        if let floatData = buffer.floatChannelData {
            cb(floatData[0], count, rate)
        }
    }
    
    engine.prepare()
    do {
        try engine.start()
        print("[Capture] Started Native Audio Capture (\(format.sampleRate)Hz)")
        return 0
    } catch {
        print("[Capture] Failed to start engine: \(error)")
        return -1
    }
}

@_cdecl("speech_helper_stop_capture")
public func speechHelperStopCapture() {
    captureEngine?.stop()
    captureEngine?.inputNode.removeTap(onBus: 0)
    captureEngine = nil
    print("[Capture] Stopped")
}

// MARK: - Tahoe FFI

@_cdecl("tahoe_helper_init")
public func tahoeHelperInit(_ localePtr: UnsafePointer<CChar>?, timeout: Double) -> Int32 {
    guard #available(macOS 26.0, *) else { return kErrOsVersion }
    // Update global timeout just in case it wasn't set by speech_helper_init
    speechTimeout = timeout
    
    if tahoeSession == nil {
        tahoeSession = TahoeSession()
    }
    return kSuccess
}

@_cdecl("tahoe_helper_start")
public func tahoeHelperStart(_ localePtr: UnsafePointer<CChar>?) -> Int32 {
    guard #available(macOS 26.0, *) else { return kErrOsVersion }
    let localeStr = localePtr != nil ? String(cString: localePtr!) : "ja-JP"
    let locale = Locale(identifier: localeStr)
    print("[Tahoe] FFI Start: \(localeStr)")
    if tahoeSession == nil { tahoeSession = TahoeSession() }
    tahoeSession?.start(locale: locale)
    return kSuccess
}

@_cdecl("tahoe_helper_stop")
public func tahoeHelperStop() {
    guard #available(macOS 26.0, *) else { return }
    tahoeSession?.stop()
}
