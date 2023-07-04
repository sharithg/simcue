import http from "k6/http";
import { check, sleep } from "k6";
import { SharedArray } from "k6/data";

// Test configuration
export const options = {
  thresholds: {
    // Assert that 99% of requests finish within 3000ms.
    http_req_duration: ["p(99) < 3000"],
  },
  // Ramp the number of virtual users up and down
  stages: [
    { duration: "30s", target: 15 },
    { duration: "1m", target: 15 },
    { duration: "20s", target: 0 },
  ],
};

const data = JSON.parse(open("./large.json"));

// Simulated user behavior
export default function () {
  const url_go = "http://0.0.0.0:3333/push";
  const url_rust = "http://127.0.0.1:4433/enqueue";
  const payload = JSON.stringify({
    priority: Math.floor(Math.random() * 10),
    data,
  });

  const params = {
    headers: {
      "Content-Type": "application/json",
    },
  };

  const res = http.post(url_rust, payload, params);
  // Validate response status
  check(res, { "status was 200": (r) => r.status == 200 });
  sleep(1);
}
