import { createApp, nextTick } from "vue";
import App from "./App.vue";
import { shadcnUi } from "./components/ui";
import { createFrontendBootController } from "./lib/frontendBoot";
import "./style.css";

const app = createApp(App);
const frontendBoot = createFrontendBootController(() => window.__LINGLUX_BOOT__);

app.config.errorHandler = (error) => {
  frontendBoot.fail(error);
  console.error("Linglux frontend failed to start", error);
};

app.use(shadcnUi);
app.mount("#app");

void nextTick(() => {
  frontendBoot.ready();
});
