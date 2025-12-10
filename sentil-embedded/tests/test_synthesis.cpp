#include "Sentil.h"

#include <cmath>
#include <cstdint>
#include <cstdio>

#include "sentil_test.hpp"

static void numerics() {
    double spd[4] = {2, 0, 0, 2};
    double rhs[2] = {4, 6};
    double x[2];
    CHECK(sentil_embedded_solve_spd(spd, 2, rhs, x) == SENTIL_EMBEDDED_OK);
    CHECK_CLOSE(x[0], 2.0, 1e-9);
    CHECK_CLOSE(x[1], 3.0, 1e-9);

    double sym[4] = {3, 0, 0, 5};
    double values[2];
    double vectors[4];
    CHECK(sentil_embedded_symmetric_eigen(sym, 2, values, vectors) == SENTIL_EMBEDDED_OK);
    double lo = values[0] < values[1] ? values[0] : values[1];
    double hi = values[0] < values[1] ? values[1] : values[0];
    CHECK_CLOSE(lo, 3.0, 1e-9);
    CHECK_CLOSE(hi, 5.0, 1e-9);

    double p[1] = {1.0};
    double q[1] = {0.0};
    double g[1] = {-1.0};
    double h[1] = {-1.0};
    double u[1];
    CHECK(sentil_embedded_solve_qp(p, 1, q, g, h, 1, u) == SENTIL_EMBEDDED_OK);
    CHECK_CLOSE(u[0], 1.0, 1e-4);

    double vals[3] = {1.0, 3.0, 2.0};
    CHECK_CLOSE(sentil_embedded_soft_min(vals, 3, 100.0), 1.0, 0.5);
    CHECK_CLOSE(sentil_embedded_soft_max(vals, 3, 100.0), 3.0, 0.5);
}

static void safety_filter_clamps() {
    double lower[3] = {-1, -1, -1};
    double upper[3] = {1, 1, 1};
    sentil_embedded_bounds_t* bounds = nullptr;
    CHECK(sentil_embedded_bounds_create(lower, upper, 3, &bounds) == SENTIL_EMBEDDED_OK);
    sentil_embedded_safety_filter_t* filter = nullptr;
    CHECK(sentil_embedded_safety_filter_create(bounds, &filter) == SENTIL_EMBEDDED_OK);

    double nominal[3] = {2.0, 0.5, -3.0};
    double out[3];
    CHECK(sentil_embedded_safety_filter_filter(filter, nominal, 3, nullptr, nullptr, 0, out) ==
          SENTIL_EMBEDDED_OK);
    CHECK_CLOSE(out[0], 1.0, 1e-9);
    CHECK_CLOSE(out[1], 0.5, 1e-9);
    CHECK_CLOSE(out[2], -1.0, 1e-9);

    CHECK(sentil_embedded_safety_filter_filter(filter, nominal, 2, nullptr, nullptr, 0, out) ==
          SENTIL_EMBEDDED_INVALID_CONFIG);
    sentil_embedded_safety_filter_destroy(filter);
}

static void open_loop_synthesis() {
    double a[1] = {1.0};
    double b[1] = {1.0};
    double x0[1] = {1.0};
    const char* vars[1] = {"x"};
    sentil_embedded_model_t* model = nullptr;
    CHECK(sentil_embedded_linear_model_create(a, 1, b, 1, x0, vars, 1.0, 3, &model) ==
          SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_model_input_dimension(model) == 3);

    sentil_embedded_formula_t* spec = nullptr;
    CHECK(sentil_embedded_formula_create("always (x > 0)", &spec) == SENTIL_EMBEDDED_OK);

    double lower[3] = {-1, -1, -1};
    double upper[3] = {1, 1, 1};
    sentil_embedded_bounds_t* bounds = nullptr;
    CHECK(sentil_embedded_bounds_create(lower, upper, 3, &bounds) == SENTIL_EMBEDDED_OK);

    double input[3];
    double robustness = 0.0;
    bool holds = false;
    CHECK(sentil_embedded_synthesize(model, spec, bounds, 0, 0, input, &robustness, &holds) ==
          SENTIL_EMBEDDED_OK);
    std::printf("synthesize: robustness=%.4f holds=%d input=[%.3f %.3f %.3f]\n", robustness,
                (int)holds, input[0], input[1], input[2]);
    CHECK(holds);
    CHECK(robustness >= 0.99);

    sentil_embedded_bounds_destroy(bounds);
    sentil_embedded_formula_destroy(spec);
    sentil_embedded_model_destroy(model);
}

static void online_controller() {
    double a[1] = {1.0};
    double b[1] = {1.0};
    double x0[1] = {0.0};
    const char* vars[1] = {"x"};
    sentil_embedded_model_t* model = nullptr;
    CHECK(sentil_embedded_linear_model_create(a, 1, b, 1, x0, vars, 1.0, 5, &model) ==
          SENTIL_EMBEDDED_OK);
    sentil_embedded_formula_t* spec = nullptr;
    CHECK(sentil_embedded_formula_create("always (x > 0)", &spec) == SENTIL_EMBEDDED_OK);
    double lower[5] = {-1, -1, -1, -1, -1};
    double upper[5] = {1, 1, 1, 1, 1};
    sentil_embedded_bounds_t* bounds = nullptr;
    CHECK(sentil_embedded_bounds_create(lower, upper, 5, &bounds) == SENTIL_EMBEDDED_OK);

    // create consumes model and spec; bounds stays ours to free.
    sentil_embedded_controller_t* controller = nullptr;
    CHECK(sentil_embedded_controller_create(model, spec, 1, 200, bounds, &controller) ==
          SENTIL_EMBEDDED_OK);
    sentil_embedded_bounds_destroy(bounds);

    double state[1] = {0.5};
    double input[1] = {0.0};
    CHECK(sentil_embedded_controller_control(controller, state, 1, input) == SENTIL_EMBEDDED_OK);
    std::printf("controller: input=%.4f\n", input[0]);
    CHECK(std::isfinite(input[0]));
    CHECK(input[0] >= -1.0 - 1e-9 && input[0] <= 1.0 + 1e-9);
    sentil_embedded_controller_destroy(controller);
}

static void error_paths() {
    double out[1];
    CHECK(sentil_embedded_solve_spd(nullptr, 1, nullptr, out) == SENTIL_EMBEDDED_NULL_POINTER);
    // a non-square model fails with a config error, not a fault
    double a[2] = {1, 1};
    double b[1] = {1};
    double x0[1] = {0};
    const char* vars[1] = {"x"};
    sentil_embedded_model_t* model = nullptr;
    sentil_embedded_status_t st =
        sentil_embedded_linear_model_create(a, 1, b, 1, x0, vars, 1.0, 0, &model);
    CHECK(st == SENTIL_EMBEDDED_INVALID_CONFIG);
    CHECK(model == nullptr);
    sentil_embedded_formula_destroy(nullptr);
    sentil_embedded_controller_destroy(nullptr);
}

int main() {
    static std::uint8_t heap[1 << 16];
    sentil_embedded_init(heap, sizeof(heap));

    numerics();
    safety_filter_clamps();
    open_loop_synthesis();
    online_controller();
    error_paths();

    return sentil_report("test_synthesis");
}