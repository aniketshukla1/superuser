use super::super::*;

/// Where to click an element: the middle of its first visible box, after scrolling it in.
pub(in crate::core::agentbrowser::computer) async fn centre(cdp: &Cdp, session: &str, el: &Element) -> Result<(f64, f64), String> {
    let node = json!({"backendNodeId": el.backend_node_id});
    let _ = cdp.call(Some(session), "DOM.scrollIntoViewIfNeeded", node.clone(), ACTION_TIMEOUT).await;
    let quads = cdp.call(Some(session), "DOM.getContentQuads", node, ACTION_TIMEOUT).await.map_err(words)?;
    let quad = quads["quads"]
        .as_array()
        .and_then(|qs| {
            qs.iter().find_map(|q| {
                let p: Vec<f64> = q.as_array()?.iter().filter_map(|n| n.as_f64()).collect();
                if p.len() != 8 {
                    return None;
                }
                let (w, h) = ((p[2] - p[0]).abs().max((p[4] - p[6]).abs()), (p[5] - p[1]).abs().max((p[7] - p[3]).abs()));
                (w >= 1.0 && h >= 1.0).then(|| ((p[0] + p[2] + p[4] + p[6]) / 4.0, (p[1] + p[3] + p[5] + p[7]) / 4.0))
            })
        })
        .ok_or_else(|| format!("“{}” isn't visible on the page, so it can't be clicked. Scroll, or take a new snapshot.", el.name))?;
    Ok(quad)
}

pub(in crate::core::agentbrowser::computer) async fn press_key(cdp: &Cdp, session: &str, p: &keys::Press) -> Result<(), String> {
    let mut down = json!({
        "type": if p.text.is_some() { "keyDown" } else { "rawKeyDown" },
        "key": p.key, "code": p.code,
        "windowsVirtualKeyCode": p.vk, "nativeVirtualKeyCode": p.vk,
        "modifiers": p.modifiers,
    });
    if let Some(t) = &p.text {
        down["text"] = json!(t);
    }
    cdp.call(Some(session), "Input.dispatchKeyEvent", down, ACTION_TIMEOUT).await.map_err(words)?;
    cdp.call(
        Some(session),
        "Input.dispatchKeyEvent",
        json!({"type": "keyUp", "key": p.key, "code": p.code, "windowsVirtualKeyCode": p.vk, "nativeVirtualKeyCode": p.vk, "modifiers": p.modifiers}),
        ACTION_TIMEOUT,
    )
    .await
    .map_err(words)?;
    Ok(())
}
