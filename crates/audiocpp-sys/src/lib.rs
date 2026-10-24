//! Runtime-loaded bindings for the audio.cpp C API (`libaudiocpp`).
//!
//! Hand-written `extern "C"` declarations only — no build-time C++ compile;
//! the shared library is located at runtime via `libloading`.
//! Implemented in plan Step 7 against `audio.cpp/include/audiocpp.h` @ v0.9.2.
