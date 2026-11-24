use std::collections::HashMap;

use gpui::{App, AppContext, Context, Entity, Global, Subscription, Window};
use gpui_pages::router;
use services::{AppState, instance::ServerInstance};

use super::server::screen::ServerScreen;

pub struct ServerSessions(pub Entity<ServerSessionStore>);

impl Global for ServerSessions {}

impl ServerSessions {
    pub fn init(cx: &mut App) {
        if cx.try_global::<Self>().is_none() {
            let store = cx.new(ServerSessionStore::new);
            cx.set_global(Self(store));
        }
    }
}

pub struct ServerSessionStore {
    screens: HashMap<String, Entity<ServerScreen>>,
    subscriptions: HashMap<String, Subscription>,
}

impl ServerSessionStore {
    fn new(cx: &mut Context<Self>) -> Self {
        let ordered_ids = sorted_server_ids(cx.global::<AppState>().instance_service.servers());
        cx.update_global::<AppState, _>(|state, _| {
            if state.topology.ensure_positions(&ordered_ids) {
                state.save_topology();
            }
        });
        Self {
            screens: HashMap::new(),
            subscriptions: HashMap::new(),
        }
    }

    pub fn screen(&self, id: &str) -> Option<Entity<ServerScreen>> {
        self.screens.get(id).cloned()
    }

    pub fn is_running(&self, id: &str, cx: &App) -> bool {
        self.screens
            .get(id)
            .is_some_and(|screen| screen.read(cx).running)
    }

    fn ensure_screen(&mut self, server: ServerInstance, cx: &mut Context<Self>) {
        if self.screens.contains_key(&server.id) {
            return;
        }
        let id = server.id.clone();
        let screen = cx.new(|cx| ServerScreen::new(server, cx));
        let sub = cx.observe(&screen, |_, _, cx| cx.notify());
        self.subscriptions.insert(id.clone(), sub);
        self.screens.insert(id, screen);
    }

    pub fn open_server(
        &mut self,
        server: ServerInstance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = server.id.clone();
        if let Some(screen) = self.screens.get(&id) {
            screen.update(cx, |screen, cx| screen.refresh_console(cx));
        } else {
            self.ensure_screen(server, cx);
        }
        navigate_to_server(&id, window, cx);
        cx.notify();
    }

    pub fn open_created_server(
        &mut self,
        server: ServerInstance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = server.id.clone();
        self.ensure_screen(server, cx);
        let ordered_ids = sorted_server_ids(cx.global::<AppState>().instance_service.servers());
        cx.update_global::<AppState, _>(|state, _| {
            if state.topology.ensure_positions(&ordered_ids) {
                state.save_topology();
            }
        });
        navigate_to_server(&id, window, cx);
        cx.notify();
    }

    pub fn on_instance_deleted(&mut self, id: &str, cx: &mut Context<Self>) {
        self.screens.remove(id);
        self.subscriptions.remove(id);
        cx.update_global::<AppState, _>(|state, _| {
            if state.topology.remove_server(id) {
                state.save_topology();
            }
        });
        cx.notify();
    }
}

fn navigate_to_server(id: &str, window: &mut Window, cx: &mut Context<ServerSessionStore>) {
    let target = format!("/servers/{id}");
    if let Some(manager) = router(window, cx) {
        manager.update(cx, |manager, cx| {
            if manager.current_path() != target
                && let Err(error) = manager.navigate(&target, cx)
            {
                eprintln!("{error}");
            }
        });
    }
}

pub fn sorted_server_ids(servers: &[ServerInstance]) -> Vec<String> {
    let mut sorted = servers.to_vec();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted.into_iter().map(|server| server.id).collect()
}
