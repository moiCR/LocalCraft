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
                    name: "Home",
                    component: HomeView,
                },
                {
                    path: "servers",
                    name: "Servers",
                    component: () => import("../views/ServersView.vue")
                }
            ],
        },
    ],  
});

export default router;