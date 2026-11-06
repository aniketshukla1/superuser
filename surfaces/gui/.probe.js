(async () => {
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  let ok = null;
  for (let i = 0; i < 150 && !ok; i++) { ok = document.querySelector(".sidebar .list-row"); if (!ok) await wait(200); }
  const t = window.__SUPERUSER_API_TOKEN__;
  const b = await (await fetch(`${window.__SUPERUSER_HTTP__}/v1/sessions`, { headers: { "X-Superuser-Token": t } })).json();
  window.location.hash = "#/chat/" + (b.sessions || b.items || []).find((x) => (x.messages || 0) > 3).session_id;
  await wait(2800);
  const box = document.querySelector(".composer"), ta = box.querySelector("textarea");
  const send = box.querySelector('[aria-label="Send"]');
  const B = (e) => e.getBoundingClientRect();
  const br = B(box), tr = B(ta), sr = send ? B(send) : null;
  const c = getComputedStyle(ta);
  // Where the TEXT actually sits, counting the textarea's own padding.
  const textTop = tr.top - br.top + parseFloat(c.paddingTop);
  const textBottom = br.bottom - tr.bottom + parseFloat(c.paddingBottom);
  return {
    boxH: Math.round(br.height),
    text: { top: Math.round(textTop), bottom: Math.round(textBottom), left: Math.round(tr.left - br.left) },
    send: sr ? { top: Math.round(sr.top - br.top), right: Math.round(br.right - sr.right), bottom: Math.round(br.bottom - sr.bottom) } : null,
    gapTextToSend: sr ? Math.round(sr.left - tr.right) : null,
    even: sr ? (Math.round(textTop) === Math.round(textBottom) && Math.round(sr.top - br.top) === Math.round(br.bottom - sr.bottom)) : null,
  };
})()
