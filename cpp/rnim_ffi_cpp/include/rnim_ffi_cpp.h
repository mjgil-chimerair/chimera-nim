#ifndef RNIM_FFI_CPP_H
#define RNIM_FFI_CPP_H

#include <string>

namespace rnim {

// Forward declarations
struct MirModule;
struct CompilationResult;

// Simple LLVM bridge placeholder
class LlvmBridge {
public:
    LlvmBridge();
    ~LlvmBridge();

    // Initialize LLVM
    bool init();

    // Compile MIR to object code
    CompilationResult compile(const MirModule& mir);

    // Get last error message
    std::string last_error() const;
};

// Compilation result
struct CompilationResult {
    bool success;
    std::string object_code;
    std::string error_message;
};

// Create a new LLVM bridge instance
LlvmBridge* create_llvm_bridge();

} // namespace rnim

#endif // RNIM_FFI_CPP_H