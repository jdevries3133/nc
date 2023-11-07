use super::{
    super::{components::Component, routes::Route},
    models,
};

impl Component for models::PropVal {
    fn render(&self) -> String {
        match self.value {
            models::Value::Bool(val) => {
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
            models::Value::Int(val) => {
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
        }
    }
}
