// This file provides missing MSVC 14.40 STL internal symbols
// that clang-cl failed to inline when compiling the pre-built Skia library.
// See: https://github.com/rust-skia/rust-skia/issues

#include <cstring>

// Helper for std::search with trivial char types (from <xutility>)
const char* pastebridge_std_search_1(
    const char* _First1, const char* _Last1,
    const char* _First2, const char* _Last2) noexcept {
    const auto _Count2 = _Last2 - _First2;
    if (_Count2 == 0) return _First1;
    const auto _Count1 = _Last1 - _First1;
    for (auto _Ptr = _First1; _Ptr + _Count2 <= _Last1; ++_Ptr) {
        if (std::memcmp(_Ptr, _First2, _Count2) == 0) return _Ptr;
    }
    return _Last1;
}

// Helper for std::find_first_of with trivial char types (from <algorithm>)
const char* pastebridge_std_find_first_of_trivial_pos_1(
    const char* _First1, const char* _Last1,
    const char* _First2, const char* _Last2) noexcept {
    for (auto _Ptr = _First1; _Ptr != _Last1; ++_Ptr) {
        if (std::memchr(_First2, static_cast<unsigned char>(*_Ptr), _Last2 - _First2)) {
            return _Ptr;
        }
    }
    return _Last1;
}