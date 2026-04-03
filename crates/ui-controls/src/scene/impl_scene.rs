use app_kit_lite::{
    menu::{ModifierFlags, NSMenu},
    NSPasteboard, NS_PASTEBOARD_TYPE_STRING,
};
use graphics_core::{
    Color, Font, FontManager, Point, Rect, Renderer as _, SoftwareRenderer, Surface,
};
use ui_view::layout::{LayoutConstraint, LayoutEngine};
use window_backend::KeyCode;

use crate::{
    Button, Checkbox, ComboBox, ImageView, Label, ProgressBar, RadioButton, ScrollView, Slider,
    TabView, TextField,
};
use super::AppScene;

impl AppScene {
    pub fn new() -> Self {
        Self {
            background: Color::WHITE,
            labels: Vec::new(),
            buttons: Vec::new(),
            textfields: Vec::new(),
            checkboxes: Vec::new(),
            radiobuttons: Vec::new(),
            sliders: Vec::new(),
            progressbars: Vec::new(),
            comboboxes: Vec::new(),
            tabviews: Vec::new(),
            imageviews: Vec::new(),
            scrollviews: Vec::new(),
            on_button_click: None,
            main_menu: None,
            focused_textfield: None,
            modifier_flags: ModifierFlags::default(),
            font_manager: FontManager::new(),
            constraints: Vec::new(),
        }
    }

    // ── Builder API ────────────────────────────────────

    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    pub fn add_label(&mut self, label: Label) {
        self.labels.push(label);
    }

    pub fn add_button(&mut self, button: Button) {
        self.buttons.push(button);
    }

    pub fn add_textfield(&mut self, tf: TextField) {
        self.textfields.push(tf);
    }

    pub fn add_checkbox(&mut self, cb: Checkbox) {
        self.checkboxes.push(cb);
    }

    pub fn add_radiobutton(&mut self, rb: RadioButton) {
        self.radiobuttons.push(rb);
    }

    pub fn add_slider(&mut self, slider: Slider) {
        self.sliders.push(slider);
    }

    pub fn add_progressbar(&mut self, pb: ProgressBar) {
        self.progressbars.push(pb);
    }

    pub fn add_combobox(&mut self, cb: ComboBox) {
        self.comboboxes.push(cb);
    }

    pub fn add_tabview(&mut self, tv: TabView) {
        self.tabviews.push(tv);
    }

    pub fn add_imageview(&mut self, iv: ImageView) {
        self.imageviews.push(iv);
    }

    pub fn add_scrollview(&mut self, sv: ScrollView) {
        self.scrollviews.push(sv);
    }

    pub fn on_button_click(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>) + 'static,
    ) {
        self.on_button_click = Some(Box::new(handler));
    }

    pub fn set_main_menu(&mut self, menu: NSMenu) {
        self.main_menu = Some(menu);
    }

    /// Add a layout constraint.
    pub fn add_constraint(&mut self, constraint: LayoutConstraint) {
        self.constraints.push(constraint);
    }

    /// Add multiple layout constraints.
    pub fn add_constraints(&mut self, constraints: impl IntoIterator<Item = LayoutConstraint>) {
        self.constraints.extend(constraints);
    }

    /// Solve layout constraints and update frames of all controls that have
    /// a `layout_id` set. Returns the number of constraint conflicts (0 = success).
    pub fn solve_layout(&mut self, width: f64, height: f64) -> usize {
        if self.constraints.is_empty() {
            return 0;
        }
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, width, height));
        engine.add_constraints(self.constraints.iter().cloned());
        match engine.solve() {
            Ok(frames) => {
                for label in &mut self.labels {
                    if let Some(vid) = label.view().layout_id {
                        if let Some(rect) = frames.get(&vid) {
                            label.view_mut().set_frame(*rect);
                        }
                    }
                }
                for button in &mut self.buttons {
                    if let Some(vid) = button.view().layout_id {
                        if let Some(rect) = frames.get(&vid) {
                            button.view_mut().set_frame(*rect);
                        }
                    }
                }
                for tf in &mut self.textfields {
                    if let Some(vid) = tf.view().layout_id {
                        if let Some(rect) = frames.get(&vid) {
                            tf.view_mut().set_frame(*rect);
                        }
                    }
                }
                0
            }
            Err(conflicts) => conflicts.len(),
        }
    }

    // ── Rendering ──────────────────────────────────────

    /// Height of the menu bar (0 if no menu is set).
    pub fn menu_bar_height(&self) -> f64 {
        if self.main_menu.is_some() {
            22.0
        } else {
            0.0
        }
    }

    /// Render the entire scene into a surface of the given logical dimensions.
    pub fn render(&mut self, width: u32, height: u32) -> Surface {
        let mut surface = Surface::new(width, height);
        {
            let mut renderer = SoftwareRenderer::with_fonts(&mut surface, &self.font_manager);
            renderer.clear(self.background);
            self.draw_menu_bar(&mut renderer, width as f64);
            for label in &mut self.labels {
                label.draw(&mut renderer);
            }
            for button in &mut self.buttons {
                button.draw(&mut renderer);
            }
            for tf in &mut self.textfields {
                tf.draw(&mut renderer);
            }
            for cb in &mut self.checkboxes {
                cb.draw(&mut renderer);
            }
            for rb in &mut self.radiobuttons {
                rb.draw(&mut renderer);
            }
            for slider in &mut self.sliders {
                slider.draw(&mut renderer);
            }
            for pb in &mut self.progressbars {
                pb.draw(&mut renderer);
            }
            for cb in &mut self.comboboxes {
                cb.draw(&mut renderer);
            }
            for tv in &mut self.tabviews {
                tv.draw(&mut renderer);
            }
            for iv in &mut self.imageviews {
                iv.draw(&mut renderer);
            }
        }
        surface
    }

    /// Draw the menu bar into the renderer, if a menu is set.
    fn draw_menu_bar(&self, renderer: &mut SoftwareRenderer<'_>, width: f64) {
        let Some(ref menu) = self.main_menu else {
            return;
        };
        let mb_height = self.menu_bar_height();
        let menu_bg = Color::new(0.95, 0.95, 0.95, 1.0);
        let menu_rect = Rect::new(0.0, 0.0, width, mb_height);
        renderer.fill_rect(menu_rect, menu_bg);
        let font = Font::system(12.0);
        let sys_font = self.font_manager.system();
        let metrics = sys_font.metrics(font.size);
        let text_y = (mb_height - metrics.line_height()) / 2.0 + metrics.ascent;
        let mut x = 12.0;
        for item in menu.items() {
            if !item.is_separator() {
                renderer.draw_text(item.title(), Point::new(x, text_y), &font, Color::BLACK);
                let item_width = sys_font.measure_text(item.title(), font.size);
                x += item_width + 16.0;
            }
        }
        renderer.draw_line(
            Point::new(0.0, mb_height),
            Point::new(width, mb_height),
            Color::new(0.8, 0.8, 0.8, 1.0),
            1.0,
        );
    }

    // ── Input handling ─────────────────────────────────

    /// Handle a scroll event. Offsets all ScrollView content_offsets.
    /// Returns `true` if any scrollview consumed the scroll.
    pub fn handle_scroll(&mut self, dx: f64, dy: f64) -> bool {
        if self.scrollviews.is_empty() {
            return false;
        }
        for sv in &mut self.scrollviews {
            sv.scroll_by(dx, dy);
        }
        true
    }

    /// Handle a click at the given point. Updates textfield focus and
    /// fires button actions. Returns `true` if something was handled.
    pub fn handle_click(&mut self, point: Point) -> bool {
        // Check textfield focus
        self.focused_textfield = self.textfields.iter().enumerate().find_map(|(i, tf)| {
            if tf.is_editable() && tf.view().frame.contains(point) {
                Some(i)
            } else {
                None
            }
        });

        let clicked_idx = self.buttons.iter().enumerate().find_map(|(i, btn)| {
            if btn.is_enabled() && btn.view().frame.contains(point) {
                Some(i)
            } else {
                None
            }
        });

        if let Some(idx) = clicked_idx {
            self.buttons[idx].perform_click();
            if let Some(ref mut handler) = self.on_button_click {
                handler(idx, &mut self.labels, &mut self.textfields);
            }
            return true;
        }
        false
    }

    /// Update the current modifier key state.
    pub fn set_modifier_flags(&mut self, flags: ModifierFlags) {
        self.modifier_flags = flags;
    }

    /// Handle a key press. Checks menu shortcuts first, then dispatches
    /// keyboard navigation (Tab cycles textfield focus, Space/Return activates
    /// focused button, Left/Right adjusts focused slider).
    /// Returns `true` if anything was consumed.
    pub fn handle_key_down(&mut self, key: KeyCode) -> bool {
        if self.handle_textfield_shortcut(key) {
            return true;
        }

        // Menu key equivalents first
        if let Some(ref mut menu) = self.main_menu {
            if menu.perform_key_equivalent(key, self.modifier_flags) {
                return true;
            }
        }

        match key {
            KeyCode::Tab => {
                // Cycle focus between textfields
                if self.textfields.is_empty() {
                    return false;
                }
                self.focused_textfield = Some(match self.focused_textfield {
                    Some(idx) => (idx + 1) % self.textfields.len(),
                    None => 0,
                });
                true
            }
            KeyCode::Space | KeyCode::Return => {
                // Activate the first enabled button (simple focus: first button)
                let clicked_idx = self.buttons.iter().position(|b| b.is_enabled());
                if let Some(idx) = clicked_idx {
                    self.buttons[idx].perform_click();
                    if let Some(ref mut handler) = self.on_button_click {
                        handler(idx, &mut self.labels, &mut self.textfields);
                    }
                    return true;
                }
                false
            }
            KeyCode::Left | KeyCode::Right => {
                // Adjust first slider by step (1% of range)
                if let Some(slider) = self.sliders.first_mut() {
                    if slider.is_enabled() {
                        let step = (slider.max_value() - slider.min_value()) * 0.01;
                        let delta = if key == KeyCode::Left { -step } else { step };
                        slider.set_value(slider.value() + delta);
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Handle character input (typed text). Inserts into the focused
    /// text field. Returns `true` if consumed.
    pub fn handle_character_input(&mut self, text: &str) -> bool {
        let Some(idx) = self.focused_textfield else {
            return false;
        };
        if idx >= self.textfields.len() {
            return false;
        }
        for ch in text.chars() {
            self.textfields[idx].insert_char(ch);
        }
        true
    }

    /// Handle backspace in the focused text field. Returns `true` if consumed.
    pub fn handle_backspace(&mut self) -> bool {
        let Some(idx) = self.focused_textfield else {
            return false;
        };
        if idx >= self.textfields.len() {
            return false;
        }
        self.textfields[idx].delete_backward();
        true
    }

    // ── Inspection (for headless testing) ──────────────

    pub fn label_text(&self, idx: usize) -> &str {
        self.labels[idx].text()
    }

    pub fn textfield_text(&self, idx: usize) -> &str {
        self.textfields[idx].text()
    }

    pub fn button_title(&self, idx: usize) -> &str {
        self.buttons[idx].title()
    }

    pub fn label_count(&self) -> usize {
        self.labels.len()
    }

    pub fn button_count(&self) -> usize {
        self.buttons.len()
    }

    pub fn textfield_count(&self) -> usize {
        self.textfields.len()
    }

    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    pub fn buttons(&self) -> &[Button] {
        &self.buttons
    }

    pub fn textfields(&self) -> &[TextField] {
        &self.textfields
    }

    pub fn checkboxes(&self) -> &[Checkbox] {
        &self.checkboxes
    }

    pub fn checkbox_count(&self) -> usize {
        self.checkboxes.len()
    }

    pub fn radiobuttons(&self) -> &[RadioButton] {
        &self.radiobuttons
    }

    pub fn radiobutton_count(&self) -> usize {
        self.radiobuttons.len()
    }

    pub fn sliders(&self) -> &[Slider] {
        &self.sliders
    }

    pub fn slider_count(&self) -> usize {
        self.sliders.len()
    }

    pub fn progressbars(&self) -> &[ProgressBar] {
        &self.progressbars
    }

    pub fn progressbar_count(&self) -> usize {
        self.progressbars.len()
    }

    pub fn comboboxes(&self) -> &[ComboBox] {
        &self.comboboxes
    }

    pub fn combobox_count(&self) -> usize {
        self.comboboxes.len()
    }

    pub fn tabviews(&self) -> &[TabView] {
        &self.tabviews
    }

    pub fn tabview_count(&self) -> usize {
        self.tabviews.len()
    }

    pub fn imageviews(&self) -> &[ImageView] {
        &self.imageviews
    }

    pub fn imageview_count(&self) -> usize {
        self.imageviews.len()
    }

    pub fn scrollviews(&self) -> &[ScrollView] {
        &self.scrollviews
    }

    pub fn scrollview_count(&self) -> usize {
        self.scrollviews.len()
    }

    pub fn focused_textfield_index(&self) -> Option<usize> {
        self.focused_textfield
    }

    fn handle_textfield_shortcut(&mut self, key: KeyCode) -> bool {
        if !self.modifier_flags.command || self.modifier_flags.control || self.modifier_flags.option
        {
            return false;
        }

        let Some(idx) = self.focused_textfield else {
            return false;
        };
        if idx >= self.textfields.len() {
            return false;
        }

        match key {
            KeyCode::A => {
                self.textfields[idx].select_all();
                true
            }
            KeyCode::C => {
                let Some(selected_text) = self.textfields[idx].copy_selection() else {
                    return false;
                };
                let mut pasteboard = NSPasteboard::general();
                pasteboard.clear_contents();
                pasteboard.set_string(&selected_text, NS_PASTEBOARD_TYPE_STRING)
            }
            KeyCode::X => {
                let Some(selected_text) = self.textfields[idx].cut_selection() else {
                    return false;
                };
                let mut pasteboard = NSPasteboard::general();
                pasteboard.clear_contents();
                pasteboard.set_string(&selected_text, NS_PASTEBOARD_TYPE_STRING)
            }
            KeyCode::V => {
                let pasteboard = NSPasteboard::general();
                let Some(text) = pasteboard.string_for_type(NS_PASTEBOARD_TYPE_STRING) else {
                    return false;
                };
                self.textfields[idx].paste_text(text)
            }
            _ => false,
        }
    }
}

