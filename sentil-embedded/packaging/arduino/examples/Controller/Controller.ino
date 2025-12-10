// Controller: plan and run a receding-horizon controller on the board.
//
// The plant is a single integrator x_{t+1} = x_t + u_t, and the spec asks x to
// stay above zero. Each loop the controller plans an input from the live state
// within a fixed step budget, the plant advances under the input and a steady
// disturbance pulling it down, and both are printed over serial. This is the same
// engine the desktop tools use, planning in real time on the chip.
//
// Synthesis needs more heap than the bare monitor, so this reserves 8 KB. Build
// the archive with the synthesis feature, which is on by default.
#include <Sentil.h>

static sentil_embedded_controller_t* controller = nullptr;
static uint8_t sentil_heap[8192];
static double x = 2.0;

void setup() {
  Serial.begin(115200);
  while (!Serial) {
  }
  sentil_embedded_init(sentil_heap, sizeof(sentil_heap));

  double a[1] = {1.0};
  double b[1] = {1.0};
  double x0[1] = {2.0};
  const char* vars[1] = {"x"};
  sentil_embedded_model_t* model = nullptr;
  if (sentil_embedded_linear_model_create(a, 1, b, 1, x0, vars, 1.0, 5, &model) !=
      SENTIL_EMBEDDED_OK) {
    Serial.println("could not build the model");
    while (true) {
    }
  }
  sentil_embedded_formula_t* spec = nullptr;
  sentil_embedded_formula_create("always (x > 0)", &spec);
  double lo[5] = {-1, -1, -1, -1, -1};
  double hi[5] = {1, 1, 1, 1, 1};
  sentil_embedded_bounds_t* bounds = nullptr;
  sentil_embedded_bounds_create(lo, hi, 5, &bounds);

  // create consumes the model and spec; the bounds stay ours to free.
  if (sentil_embedded_controller_create(model, spec, 1, 150, bounds, &controller) !=
      SENTIL_EMBEDDED_OK) {
    Serial.println("could not build the controller");
    while (true) {
    }
  }
  sentil_embedded_bounds_destroy(bounds);
}

void loop() {
  double state[1] = {x};
  double u[1] = {0.0};
  if (sentil_embedded_controller_control(controller, state, 1, u) == SENTIL_EMBEDDED_OK) {
    x += u[0] - 0.3;  // 0.3 is the disturbance
    Serial.print("x=");
    Serial.print(x);
    Serial.print("  u=");
    Serial.println(u[0]);
  }
  delay(500);
}