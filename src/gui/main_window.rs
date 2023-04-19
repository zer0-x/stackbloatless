use adw::prelude::*;
use relm4::{
    actions::AccelsPlus,
    adw::traits::AdwWindowExt,
    component::{AsyncComponent, AsyncComponentParts, AsyncComponentSender},
    gtk::{
        prelude::ApplicationExt,
        traits::{GtkApplicationExt, WidgetExt},
    },
    loading_widgets::LoadingWidgets,
    prelude::*,
};

use super::componant_builders;
use crate::api::stackexchange;

const APP_NAME: &str = "StackBloatLess";

#[derive(Debug, Clone)]
pub enum AppInput {
    RequestPagesByUri(stackexchange::Uri),
    ToggleSearchEntry,
    ShowAboutWindow,
    Quit,
    ToggleSelectedTabPin,
    CloseTab,
    ClosePinnedTab,
}

pub struct AppInit {
    pub receiver: relm4::Receiver<AppInput>,
}

pub struct AppModel {
    stackexchange_client: stackexchange::StackExchange,
}

pub struct AppWidgets {
    tab_view: adw::TabView,
    header: adw::HeaderBar,
    search_button: gtk::ToggleButton,
    search_entry: gtk::SearchEntry,
    title_widget: adw::WindowTitle,
}

#[relm4::async_trait::async_trait(?Send)]
impl AsyncComponent for AppModel {
    type Init = AppInit;
    type Root = adw::Window;
    type Widgets = AppWidgets;
    type Input = AppInput;
    type CommandOutput = ();
    type Output = ();

    fn init_root() -> Self::Root {
        adw::Window::builder().title(APP_NAME).build()
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        let spinner = gtk::Spinner::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build();

        spinner.start();

        Some(LoadingWidgets::new(root, spinner))
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = AppModel {
            stackexchange_client: stackexchange::StackExchange::new(),
        };

        // Load CSS
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_bytes!("style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::StyleContext::add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        // Listen to messages sent from the main function.
        sender.oneshot_command(
            init.receiver
                .forward(sender.input_sender().to_owned(), |msg| msg),
        );

        let main_layout = gtk::Box::new(gtk::Orientation::Vertical, 5);

        root.set_content(Some(&main_layout));

        // Create header bar
        let title_widget = adw::WindowTitle::builder()
            .title(APP_NAME)
            .subtitle("Your 1000 tabs are in safe hands")
            .build();

        let header = adw::HeaderBar::builder()
            .title_widget(&title_widget)
            .show_end_title_buttons(true)
            .build();

        main_layout.append(&header);

        // Create menu actions
        relm4::new_action_group!(MenuActionGroup, "menu");
        relm4::new_stateless_action!(AboutAction, MenuActionGroup, "about");
        relm4::new_stateless_action!(QuitAction, MenuActionGroup, "quit");
        {
            let group = relm4::actions::RelmActionGroup::<MenuActionGroup>::new();

            let about_action: relm4::actions::RelmAction<AboutAction> =
                relm4::actions::RelmAction::new_stateless(
                    gtk::glib::clone!(@strong sender => move |_| {
                        sender.input(AppInput::ShowAboutWindow);
                    }),
                );
            group.add_action(&about_action);

            let quit_action: relm4::actions::RelmAction<QuitAction> =
                relm4::actions::RelmAction::new_stateless(
                    gtk::glib::clone!(@strong sender => move |_| {
                        sender.input(AppInput::Quit);
                    }),
                );
            group.add_action(&quit_action);

            root.insert_action_group("menu", Some(&group.into_action_group()))
        }

        relm4::menu! {
            main_menu: {
                "About" => AboutAction,
                "Quit" => QuitAction
            }
        }

        // Create hamburger menu
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&main_menu)
            .build();

        header.pack_start(&menu_button);

        // Search button and entry
        let search_button = gtk::ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .build();

        search_button.connect_clicked(gtk::glib::clone!(@strong sender => move |_search_button| {
            sender.input(AppInput::ToggleSearchEntry);
        }));

        header.pack_start(&search_button);

        let search_entry = gtk::SearchEntry::builder()
            // TODO: Make icon clickable to select a stackexchange site to search in.
            .placeholder_text("Enter a search term or question id")
            .build();

        search_entry.connect_activate(gtk::glib::clone!(@strong sender => move |entry| {
            let search_term = entry.text();
            // TODO: Change how search_term is parsed to support urls and terms at the same time.
            // TODO: Connect it to search api
            // TODO: Don't accept uris.
            // TODO: Support all stackexchange sites: https://api.stackexchange.com/docs/sites
            sender.input(AppInput::RequestPagesByUri(format!("stackexchange://stackoverflow/{search_term}")));
            entry.delete_text(0, search_term.len() as i32);
        }));

        // Create tab bar
        let tab_bar = adw::TabBar::builder()
            .css_classes(Vec::from(["inline".to_string()]))
            // TODO: Create a libadwaita::TabButton
            // .end_action_widget()
            .build();
        main_layout.append(&tab_bar);

        let tab_view = adw::TabView::new();
        main_layout.append(&tab_view);
        tab_bar.set_view(Some(&tab_view));

        // Create tab actions
        relm4::new_action_group!(TabActionGroup, "tab");
        relm4::new_stateless_action!(PinTabAction, TabActionGroup, "toggle_pin");
        relm4::new_stateless_action!(CloseTabAction, TabActionGroup, "close");
        {
            let group = relm4::actions::RelmActionGroup::<TabActionGroup>::new();

            let tab_pin_action: relm4::actions::RelmAction<PinTabAction> =
                relm4::actions::RelmAction::new_stateless(
                    gtk::glib::clone!(@strong sender => move |_| {
                        sender.input(AppInput::ToggleSelectedTabPin);
                    }),
                );
            group.add_action(&tab_pin_action);

            let close_tab_action: relm4::actions::RelmAction<CloseTabAction> =
                relm4::actions::RelmAction::new_stateless(
                    gtk::glib::clone!(@strong sender => move |_| {
                        sender.input(AppInput::CloseTab);
                    }),
                );
            group.add_action(&close_tab_action);

            root.insert_action_group("tab", Some(&group.into_action_group()))
        }

        tab_view.connect_setup_menu(|view, page| {
            if let Some(page) = page {
                view.set_selected_page(page);
            }
        });

        relm4::menu! {
            tab_menu: {
                "Pin/Unpin" => PinTabAction,
                "Close" => CloseTabAction,
            }
        }

        relm4::main_application().set_accelerators_for_action::<CloseTabAction>(&["<Control>w"]);

        tab_view.set_menu_model(Some(&tab_menu));

        // TODO: Create a libadwaita::TabOverview

        let widgets = AppWidgets {
            tab_view,
            header,
            search_button,
            search_entry,
            title_widget,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppInput::RequestPagesByUri(uri) => {
                let questions = self
                    .stackexchange_client
                    .get_questions_from_uri(&uri)
                    .await
                    .unwrap();

                for question in questions {
                    let question_box = componant_builders::st_question(&question);

                    let tab_page = widgets.tab_view.append(
                        &gtk::ScrolledWindow::builder()
                            .child(&question_box)
                            .vexpand(true)
                            .hexpand(true)
                            .build(),
                    );

                    tab_page.set_title(&question.title);
                }
            }
            AppInput::ToggleSearchEntry => {
                if widgets.search_button.is_active() {
                    widgets.header.set_title_widget(Some(&widgets.search_entry));
                    widgets.search_entry.show();
                    widgets.search_entry.grab_focus();
                } else {
                    widgets.search_entry.hide();
                    widgets.header.set_title_widget(Some(&widgets.title_widget));
                }
            }
            AppInput::ShowAboutWindow => {
                let developers = env!("CARGO_PKG_AUTHORS")
                    .to_string()
                    .split(':')
                    .map(|author| author.to_string())
                    .collect();

                let about_window = adw::AboutWindow::builder()
                    .application_name(APP_NAME)
                    .version(env!("CARGO_PKG_VERSION"))
                    .license_type(gtk::License::Gpl30Only)
                    .comments(env!("CARGO_PKG_DESCRIPTION"))
                    .developers(developers)
                    .website(env!("CARGO_PKG_HOMEPAGE"))
                    .issue_url("https://github.com/zer0-x/stackbloatless/issues")
                    .application(&relm4::main_application())
                    .transient_for(&relm4::main_application().active_window().unwrap())
                    .build();

                about_window.add_link(
                    "Release Notes",
                    "https://github.com/zer0-x/stackbloatless/blob/main/CHANGELOG.md",
                );

                about_window.present();
            }
            AppInput::Quit => {
                relm4::main_application().quit();
            }
            AppInput::ToggleSelectedTabPin => {
                let selected_page = widgets.tab_view.selected_page().unwrap();

                widgets
                    .tab_view
                    .set_page_pinned(&selected_page, !selected_page.is_pinned())
            }
            AppInput::CloseTab => {
                let selected_page = widgets.tab_view.selected_page().unwrap();

                // Ask before closing a pinned tab
                if selected_page.is_pinned() {
                    let warning_message = adw::MessageDialog::builder()
                        .transient_for(&relm4::main_application().active_window().unwrap())
                        .heading("Close pinned tab?")
                        .body("Do you really want to close a pinned tab?")
                        .build();

                    warning_message.add_responses(&[("yes", "Yes"), ("no", "No")]);
                    warning_message.set_default_response(Some("no"));
                    warning_message
                        .set_response_appearance("yes", adw::ResponseAppearance::Destructive);

                    warning_message.show();

                    warning_message.connect_response(
                        None,
                        gtk::glib::clone!(@strong sender => move |dialog, responde| {
                            if responde == "yes" {
                                sender.input(AppInput::ClosePinnedTab);
                            }
                            dialog.close();
                        }),
                    );
                } else {
                    widgets.tab_view.close_page(&selected_page);
                }
            }
            AppInput::ClosePinnedTab => {
                let selected_page = widgets.tab_view.selected_page().unwrap();

                widgets.tab_view.set_page_pinned(&selected_page, false);
                widgets.tab_view.close_page(&selected_page);
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {}
}
