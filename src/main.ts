import { createApp, nextTick } from "vue";
import App from "./App.vue";
import { shadcnUi } from "./components/ui";
import "./style.css";

const app = createApp(App);

app.config.errorHandler = (error) => {
  window.__LINGLUX_BOOT__?.fail(error);
  console.error("Linglux frontend failed to start", error);
};

app.use(shadcnUi);
app.mount("#app");

void nextTick(() => {
  window.__LINGLUX_BOOT__?.ready();
});
