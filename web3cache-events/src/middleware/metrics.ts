import Metrics from "../models/metrics";

export function setMetrics(description: string, type: string, time: number) {
  const metrics = new Metrics({
    description: description,
    type: type,
    time: time,
    date: new Date(Date.now()),
  });
  metrics.save();
}
