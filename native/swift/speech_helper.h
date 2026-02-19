// speech_helper.h
// C header for Swift Speech Helper library

#ifndef SPEECH_HELPER_H
#define SPEECH_HELPER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Initialize the speech recognizer with Japanese locale
/// Returns: 0 on success, negative on error
int32_t speech_helper_init(void);

/// Request speech recognition authorization
/// Returns: 0 if authorized, negative on error/denied
int32_t speech_helper_request_authorization(void);

/// Callback type for receiving transcription results
/// @param text The transcribed text (UTF-8)
/// @param is_final 1 if this is a final result, 0 if partial
typedef void (*SpeechResultCallback)(const char *text, int32_t is_final);

/// Callback type for receiving errors
/// @param error The error message (UTF-8)
typedef void (*SpeechErrorCallback)(const char *error);

/// Set the callback for receiving transcription results
void speech_helper_set_result_callback(SpeechResultCallback callback);

/// Set the callback for receiving errors
void speech_helper_set_error_callback(SpeechErrorCallback callback);

/// Start speech recognition
/// Returns: 0 on success, negative on error
int32_t speech_helper_start(void);

/// Stop speech recognition
void speech_helper_stop(void);

/// Pump the main RunLoop
void speech_helper_tick(void);

#ifdef __cplusplus
}
#endif

#endif // SPEECH_HELPER_H
