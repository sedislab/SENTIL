// Benchmark: the per-update cost of the streaming monitor on this board.
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t sentil_heap[4096];

void setup() {
  Serial.begin(115200);
  while (!Serial) {
  }
  sentil_embedded_init(sentil_heap, sizeof(sentil_heap));
  if (monitor.begin("historically[0, 16](x > 0)") != SENTIL_EMBEDDED_OK) {
    Serial.println("could not build the monitor");
    while (true) {
    }
  }

  const int iterations = 2000;
  unsigned long total = 0;
  unsigned long worst = 0;
  unsigned long best = 0xffffffffUL;
  for (int i = 0; i < iterations; i++) {
    double packed[1] = {(double)((i % 40) - 20)};
    sentil_embedded_robustness_t r;
    unsigned long start = micros();
    monitor.update((double)i, packed, 1, r);
    unsigned long elapsed = micros() - start;
    total += elapsed;
    if (elapsed < best) {
      best = elapsed;
    }
    if (elapsed > worst) {
      worst = elapsed;
    }
  }

  Serial.print("updates: ");
  Serial.println(iterations);
  Serial.print("mean us: ");
  Serial.println((double)total / iterations);
  Serial.print("min us: ");
  Serial.println(best);
  Serial.print("max us: ");
  Serial.println(worst);
}

void loop() {
}