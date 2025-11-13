// The JNI shim. Every native method declared in NativeLib forwards here to one
// sentil_* call. The marshalling guards and the error funnel that turns a failure
// into a thrown SentilException arrive with the first fallible call; version cannot
// fail, so it stands alone as the boundary's first proof.
#include "io_github_sedislab_sentil_NativeLib.h"

#include <sentil.h>

#include <cstdint>

JNIEXPORT jintArray JNICALL Java_io_github_sedislab_sentil_NativeLib_version(JNIEnv* env, jclass) {
    uint32_t major = 0;
    uint32_t minor = 0;
    uint32_t patch = 0;
    sentil_version(&major, &minor, &patch);
    jint parts[3] = {static_cast<jint>(major), static_cast<jint>(minor), static_cast<jint>(patch)};
    jintArray out = env->NewIntArray(3);
    if (out != nullptr) {
        env->SetIntArrayRegion(out, 0, 3, parts);
    }
    return out;
}