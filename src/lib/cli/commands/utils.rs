use super::CommandContext;

/// Adjusts scroll offset based on output length and display height
pub async fn adjust_scroll_offset(ctx: &mut CommandContext<'_>) {
    let lines = ctx.output_lines.lock().await;
    *ctx.scroll_offset = 0;
    if lines.len() > ctx.output_height {
        *ctx.scroll_offset = lines.len() as u16 - ctx.output_height as u16 + 2;
    }
}
