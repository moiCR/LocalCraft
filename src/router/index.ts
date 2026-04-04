import { createMemoryHistory, createRouter } from "vue-router";
import Layout from "../layout/Layout.vue";
import HomeView from "../views/HomeView.vue";

const router = createRouter({
    history: createMemoryHistory(),
    routes: [
        {
            path: "/",
            component: Layout,
            children: [
                {
                    path: "", 
                    name: "home",
                    component: HomeView,
                },
                {
                    path: "servers",
                    name: "servers",
                    component: () => import("../views/ServersView.vue")
                },
                {
                    path: "servers/manage/:id",
                    name: "server-manage",
                    component: () => import("../views/manage/ServerManageView.vue"),
                    children: [
                        {
                            path: "console",
                            name: "server-console",
                            component: () => import("../views/manage/ServerConsoleView.vue")
                        },
                        {
                            path: "files",
                            name: "server-files",
                            component: () => import("../views/manage/ServerFilesView.vue")
                        },
                        {
                            path: "mods",
                            name: "server-mods",
                            component: () => import("../views/manage/ServerModsView.vue")
                        }
                    ]
                },
                {
                    path: "java",
                    name: "java",
                    component: () => import("../views/JavaView.vue")
                },
                {
                    path: "about",
                    name: "about",
                    component: () => import("../views/AboutView.vue")
                }
            ],
        },
    ],  
});

export default router;