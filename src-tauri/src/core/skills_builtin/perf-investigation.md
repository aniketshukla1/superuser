---
name: perf-investigation
description: Investigate a performance problem in Prometheus metrics — find the highest-confidence hotspot with evidence and links, facts apart from hypotheses
allowed-tools: metrics_find, metrics_query, metrics_range, metrics_compare, metrics_alerts, perf_report, connector_search, chart, write_file, recall_memory
---
Find where a slowdown is and prove it. Do not change anything: you can't, and a finding the
person acts on is worth more than a fix they didn't ask for.

1. **Pin the claim.** What got worse (latency, errors, throughput, saturation), for what (a page,
   an endpoint, a service), and since when. "Since yesterday's release" needs the release's time:
   ask, look in GitHub with `connector_search`, or find it in the metrics — a `*_build_info`
   metric whose `version` label changed marks a deploy.
2. **Find the metric.** `metrics_find` with the words that describe it ("http duration",
   "checkout", "requests"); then with the exact name, to see its labels — what you can group by.
   A latency histogram's percentile is `histogram_quantile(0.95, sum by (le, <label>) (rate(<name>_bucket[5m])))`.
   A percentile moves only between the histogram's buckets: one sitting flat at a bucket's edge
   (0.09 under a 0.1 bucket) can't see a change inside that bucket. Compare the average too —
   `sum by (<label>) (rate(<name>_sum[5m])) / sum by (<label>) (rate(<name>_count[5m]))`.
3. **Confirm the whole moved.** `metrics_compare` on the overall number, `change_at` the release,
   a window that suits it (1h for a sudden change, 6h to 24h for a slow one). If nothing moved
   clearly, say so: the complaint may be elsewhere, or smaller than it felt.
4. **Find where.** `metrics_compare` again, grouped by each label a hotspot could hide in —
   handler or route, pod or instance, version, region, status code. The highest-confidence
   hotspot is a series that moved clearly while its neighbours held steady; new series at the
   change (a new endpoint, a new version) are suspects too.
5. **Cross-check.** The same moment in errors (`status=~"5.."`), saturation (CPU, memory, GC,
   connection pools, queue depth) and `metrics_alerts`. A hotspot with a matching error or
   saturation shift is stronger; one without is still a finding, said so.
6. **Show it.** `chart` from the `metrics/compare-….csv` the compare kept: `source` that file, no
   `data`, `type` line, and a `note` that says what the chart shows and when the change was.
7. **Write it up**, short:
   - **Summary** — one or two sentences: what moved, where, by how much, since when.
   - **Facts** — each with its numbers as the tool gave them (before → after, the spread, the
     share of points past anything before) and its Prometheus link.
   - **Hypotheses** — labelled, each with what would confirm or rule it out (a trace of that
     endpoint, a flamegraph from that pod, the diff of that release).
   - **Ruled out** — what you compared that held steady.
   - **Next** — the one check that would settle it.

   In the conversation; when the person wants it kept or shared, also in `outputs/perf-investigation.md`.

For the weekly health report: `perf_report`, then clear shifts first with numbers and links, then
what held steady, then any signal that couldn't be read and why.
