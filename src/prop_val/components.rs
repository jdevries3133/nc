use super::models;
use crate::{components::Component, models::Value, routes::Route};

impl Component for models::PropVal {
    fn render(&self) -> String {
        match self.value {
            Value::Bool(val) => {
                let route =
                    Route::PageBoolProp(Some((self.page_id, self.prop_id)));
                let checked_state = if val { "checked" } else { "" };
                format!(
                    r#"
                    <input
                        hx-post="{route}"
                        class="justify-self-center"
                        name="value"
                        type="checkbox"
                        {checked_state}
                    />
                    "#
                )
            }
            Value::Int(val) => {
                let route =
                    Route::PageIntProp(Some((self.page_id, self.prop_id)));
                format!(
                    r#"
                    <input
                        class="rounded text-sm w-24 justify-self-center"
                        hx-post="{route}"
                        name="value"
                        type="number"
                        value="{val}"
                    />
                    "#
                )
            }
            Value::Float(val) => {
                let route =
                    Route::PageFloatProp(Some((self.page_id, self.prop_id)));
                format!(
                    r#"
                    <input
                        class="rounded text-sm w-24 justify-self-center"
                        hx-post="{route}"
                        name="value"
                        type="number"
                        step="0.01"
                        value="{val}"
                    />
                    "#
                )
            }
            Value::Date(val) => {
                let route =
                    Route::PageDateProp(Some((self.page_id, self.prop_id)));
                format!(
                    r#"
                    <input
                        class="rounded text-sm w-32 justify-self-center"
                        hx-post="{route}"
                        hx-trigger="input changed delay:1s"
                        name="value"
                        type="date"
                        value="{val}"
                    />
                    "#
                )
            }
        }
    }
}
