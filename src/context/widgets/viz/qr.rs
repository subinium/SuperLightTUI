use super::*;

impl Context {
    #[cfg(feature = "qrcode")]
    #[cfg_attr(docsrs, doc(cfg(feature = "qrcode")))]
    /// Render a QR code using half-block characters.
    pub fn qr_code(&mut self, data: impl AsRef<str>) -> Response {
        let code = match qrcode::QrCode::new(data.as_ref()) {
            Ok(code) => code,
            Err(_) => {
                let response = self.interaction();
                self.text("[QR Error]");
                return response;
            }
        };

        let modules_per_side = code.width();
        let modules = code.to_colors();
        let qr_side = modules_per_side + 2;
        let qr_width = qr_side;
        let qr_height = qr_side.div_ceil(2);
        let theme_text = self.theme.text;
        let theme_bg = self.theme.bg;

        let response = self.interaction();
        self.container()
            .w(qr_width as u32)
            .h(qr_height as u32)
            .draw(move |buf, rect| {
                let draw_w = (rect.width as usize).min(qr_width);
                let draw_h = (rect.height as usize).min(qr_height);

                for row in 0..draw_h {
                    let upper_y = row * 2;
                    let lower_y = upper_y + 1;

                    for x in 0..draw_w {
                        let resolve_module_color = |mx: usize, my: usize| -> Color {
                            let dark =
                                if mx == 0 || my == 0 || mx == qr_side - 1 || my == qr_side - 1 {
                                    false
                                } else {
                                    let inner_x = mx - 1;
                                    let inner_y = my - 1;
                                    let idx = inner_y * modules_per_side + inner_x;
                                    matches!(modules.get(idx), Some(qrcode::types::Color::Dark))
                                };

                            if dark { theme_text } else { theme_bg }
                        };

                        let upper = resolve_module_color(x, upper_y);
                        let lower = if lower_y < qr_side {
                            resolve_module_color(x, lower_y)
                        } else {
                            theme_bg
                        };

                        buf.set_char(
                            rect.x + x as u32,
                            rect.y + row as u32,
                            '▀',
                            Style::new().fg(upper).bg(lower),
                        );
                    }
                }
            });

        response
    }
}
