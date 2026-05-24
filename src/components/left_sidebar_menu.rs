use leptos::ev::MouseEvent;
use leptos::prelude::*;
use phosphor_leptos::{BARCODE, FADERS, HOUSE, INTERSECT_SQUARE, Icon, IconWeight, MAP_TRIFOLD};

#[component]
pub fn LeftSideBarMenu(active_page: RwSignal<String>) -> impl IntoView {
    view! {
        <div
            id="swarmy-tauri-left-sidebar"
            class="flex flex-col items-center w-16 h-full overflow-hidden text-gray-400 bg-gray-900 rounded gap-1 border-r border-base-100"
        >
            <svg
                class="w-8 h-8 fill-current my-3"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 32 32"
                fill="currentColor"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M 3.04,10.7C 4.74,7.81 12.18,5.92 10.99,11.19 9.19,15.57 7.14,19.85 5.35,24.25 5.03,25.35 2.41,30.52 3.77,28.76 9.17,21.67 14.58,14.58 19.99,7.49 17.84,5.42 15.69,3.35 13.55,1.27 8.35,1.81 5.92,7.1 3.04,10.7Z"
                />
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="m 30.44,10.7c -1.71,-2.89 -9.15,-4.78 -7.95,0.48 1.8,4.38 3.85,8.67 5.64,13.07 0.32,1.1 2.94,6.27 1.58,4.5C 24.3,21.67 18.89,14.58 13.49,7.49 15.63,5.42 17.78,3.35 19.93,1.27c 5.2,0.53 7.63,5.82 10.51,9.43z"
                />
            </svg>
            <div class="flex flex-col items-center mt-3 border-t border-purple-700">
                <SidebarMenuItem name="Home" active_page=active_page />
                <SidebarMenuItem name="Scan" active_page=active_page />
                <SidebarMenuItem name="Stats By Map" active_page=active_page />
                <SidebarMenuItem name="Replay List" active_page=active_page />
            </div>
            <div class="flex flex-col items-center mt-2 border-t border-purple-700">
                <SidebarMenuItem name="Config" active_page=active_page />
                <a
                    class="relative flex items-center justify-center w-12 h-12 mt-2 rounded hover:bg-gray-700 hover:text-gray-300"
                    href="#"
                >
                    <svg
                        class="w-6 h-6 stroke-current"
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
                        />
                    </svg>
                    <span class="absolute top-0 left-0 w-2 h-2 mt-2 ml-2 bg-indigo-500 rounded-full"></span>
                </a>
            </div>
        </div>
    }
}

#[component]
fn SidebarMenuItem(name: &'static str, active_page: RwSignal<String>) -> impl IntoView {
    let active_icon_class =
        "flex items-center justify-center w-12 h-12 mt-2 rounded text-gray-200 bg-gray-700";
    let inactive_icon_class = "flex items-center justify-center w-12 h-12 mt-2 rounded hover:bg-gray-700 hover:text-gray-300";
    let icon_data = match name {
        "Home" => HOUSE,
        "Scan" => BARCODE,
        "Config" => FADERS,
        "Stats By Map" => MAP_TRIFOLD,
        "Replay List" => INTERSECT_SQUARE,
        _ => HOUSE,
    };

    view! {
        <a
            id=format!("sidebar-menu-item-{}", name.replace(" ", "-").to_lowercase())
            href="#"
            class=move || {
                if active_page.get() == name { active_icon_class } else { inactive_icon_class }
            }
            title=name
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                active_page.set(name.to_string());
            }
        >
            <Icon icon=icon_data weight=IconWeight::Bold size="24px" />
        </a>
    }
}
