#ifndef SENTIL_HPP
#define SENTIL_HPP

#include <sentil.h>

#include <cstdint>

namespace sentil {

/// The version of the linked SENTIL core.
struct Version {
    std::uint32_t major;
    std::uint32_t minor;
    std::uint32_t patch;
};

inline Version version() {
    Version v{0, 0, 0};
    sentil_version(&v.major, &v.minor, &v.patch);
    return v;
}

}  // namespace sentil

#endif  // SENTIL_HPP