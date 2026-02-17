 cargo run
   Compiling dd_wcag v0.1.0 (/home/jlyvers/projects/dd_wcag)
error[E0433]: failed to resolve: use of undeclared type `Hsl`
   --> src/color.rs:150:19
    |
150 |         let hsl = Hsl::new(h, s * 100.0, l * 100.0); // palette expects percentages
    |                   ^^^ use of undeclared type `Hsl`
    |
help: consider importing this struct
    |
 14 + use palette::Hsl;
    |

warning: unused import: `widgets::*`
  --> src/main.rs:33:27
   |
33 | use ratatui::{prelude::*, widgets::*};
   |                           ^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `std::collections::HashSet`
  --> src/app.rs:13:5
   |
13 | use std::collections::HashSet;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `Clear`
  --> src/ui.rs:22:31
   |
22 |     widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
   |                               ^^^^^

error[E0599]: no function or associated item named `from_color` found for struct `palette::rgb::Rgb<S, T>` in the current scope
   --> src/color.rs:151:26
    |
151 |         let srgb = Srgb::from_color(hsl);
    |                          ^^^^^^^^^^ function or associated item not found in `palette::rgb::Rgb<Srgb, _>`
    |
note: if you're trying to build a new `palette::rgb::Rgb<Srgb, _>` consider using one of the following associated functions:
      palette::rgb::Rgb::<S, T>::new
      palette::rgb::Rgb::<S, T>::from_format
      palette::rgb::Rgb::<S, T>::from_components
      palette::rgb::Rgb::<S, u8>::from_u32
      and 2 others
   --> /home/jlyvers/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/palette-0.7.6/src/rgb/rgb.rs:216:5
    |
216 |       pub const fn new(red: T, green: T, blue: T) -> Rgb<S, T> {
    |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
259 | /     pub fn from_format<U>(color: Rgb<S, U>) -> Self
260 | |     where
261 | |         T: FromStimulus<U>,
    | |___________________________^
...
272 |       pub fn from_components((red, green, blue): (T, T, T)) -> Self {
    |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
362 | /     pub fn from_u32<O>(color: u32) -> Self
363 | |     where
364 | |         O: ComponentOrder<Rgba<S, u8>, u32>,
    | |____________________________________________^
    = help: items from traits can only be used if the trait is in scope
help: trait `FromColor` which provides `from_color` is implemented but not in scope; perhaps you want to import it
    |
 14 + use palette::FromColor;
    |
help: there is an associated function `from_color_mut` with a similar name
    |
151 |         let srgb = Srgb::from_color_mut(hsl);
    |                                    ++++

error[E0599]: no method named `to_positive_degrees` found for struct `RgbHue<T>` in the current scope
   --> src/color.rs:202:21
    |
202 |             hsl.hue.to_positive_degrees().round() as u32,
    |                     ^^^^^^^^^^^^^^^^^^^
    |
help: there is a method `into_positive_degrees` with a similar name
    |
202 |             hsl.hue.into_positive_degrees().round() as u32,
    |                     ++

error[E0599]: no method named `into_color` found for struct `palette::rgb::Rgb<S, T>` in the current scope
   --> src/color.rs:224:35
    |
224 |         let lin: LinSrgb = self.0.into_color();
    |                                   ^^^^^^^^^^
    |
   ::: /home/jlyvers/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/palette-0.7.6/src/convert/from_into_color.rs:126:8
    |
126 |     fn into_color(self) -> T;
    |        ---------- the method is available for `palette::rgb::Rgb` here
    |
    = help: items from traits can only be used if the trait is in scope
help: trait `IntoColor` which provides `into_color` is implemented but not in scope; perhaps you want to import it
    |
 14 + use palette::IntoColor;
    |
help: there is a method `into_color_mut` with a similar name
    |
224 |         let lin: LinSrgb = self.0.into_color_mut();
    |                                             ++++

warning: use of deprecated method `ratatui::Frame::<'_>::size`: use `area()` instead
  --> src/ui.rs:35:22
   |
35 |     let size = frame.size();
   |                      ^^^^
   |
   = note: `#[warn(deprecated)]` on by default

error[E0277]: `?` couldn't convert the error: `<B as ratatui::backend::Backend>::Error: Send` is not satisfied
  --> src/main.rs:86:46
   |
83 | fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
   |                                                                      ---------- required `<B as ratatui::backend::Backend>::Error: Send` because of this
...
86 |         terminal.draw(|f| ui::render(f, app))?;
   |                  ----------------------------^ `<B as ratatui::backend::Backend>::Error` cannot be sent between threads safely
   |                  |
   |                  this has type `Result<_, <B as ratatui::backend::Backend>::Error>`
   |
   = help: the trait `Send` is not implemented for `<B as ratatui::backend::Backend>::Error`
   = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
   = note: required for `anyhow::Error` to implement `From<<B as ratatui::backend::Backend>::Error>`
help: consider further restricting the associated type
   |
83 | fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> where <B as ratatui::backend::Backend>::Error: Send {
   |                                                                                 +++++++++++++++++++++++++++++++++++++++++++++++++++

error[E0277]: `?` couldn't convert the error: `<B as ratatui::backend::Backend>::Error: Sync` is not satisfied
  --> src/main.rs:86:46
   |
83 | fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
   |                                                                      ---------- required `<B as ratatui::backend::Backend>::Error: Sync` because of this
...
86 |         terminal.draw(|f| ui::render(f, app))?;
   |                  ----------------------------^ `<B as ratatui::backend::Backend>::Error` cannot be shared between threads safely
   |                  |
   |                  this has type `Result<_, <B as ratatui::backend::Backend>::Error>`
   |
   = help: the trait `Sync` is not implemented for `<B as ratatui::backend::Backend>::Error`
   = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
   = note: required for `anyhow::Error` to implement `From<<B as ratatui::backend::Backend>::Error>`
help: consider further restricting the associated type
   |
83 | fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> where <B as ratatui::backend::Backend>::Error: Sync {
   |                                                                                 +++++++++++++++++++++++++++++++++++++++++++++++++++

warning: unused variable: `bg`
   --> src/app.rs:268:32
    |
268 |         if let (Some(fg), Some(bg)) = (&self.foreground, &self.background) {
    |                                ^^ help: if this is intentional, prefix it with an underscore: `_bg`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

Some errors have detailed explanations: E0277, E0433, E0599.
For more information about an error, try `rustc --explain E0277`.
warning: `dd_wcag` (bin "dd_wcag") generated 5 warnings
error: could not compile `dd_wcag` (bin "dd_wcag") due to 6 previous errors; 5 warnings emitted