import { createEffect, createSignal, JSX, onMount } from "solid-js";
import logo from "./assets/logo.svg";
import { invoke } from "@tauri-apps/api/tauri";
import { Group } from "./Group";
import { Button } from "./Button";
import { Tab, TabBar, TabContent, Tabs } from "./Tabs";
import { Card } from "./Card";
import { listen } from "@tauri-apps/api/event";
import { parse } from "ansicolor";
import { useTheme } from "solid-styled-components";
import toast from "solid-toast";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { debounce } from "@solid-primitives/scheduled";
import { AlignedLabel } from "./AlignedLabel";

interface LogMessage {
  spans: { css: string; text: string }[];
}

type State = "Started" | "Stopped" | "NotInstalled";

interface ServiceInfo {
  state: State;
  autostart?: boolean;
  pid?: number;
  last_exit_code?: number;
}

function App() {
  const [serviceState, setServiceState] = createSignal<ServiceInfo>({
    state: "NotInstalled",
    autostart: undefined,
    pid: undefined,
    last_exit_code: undefined,
  });
  const [panelHeight, setPanelHeight] = createSignal(window.innerHeight - 220);
  const [atBottom, setAtBottom] = createSignal(true);
  const [scrollPos, setScrollPos] = createSignal(0);
  const [programmaticScroll, setProgrammaticScroll] = createSignal(false);
  const [logs, setLogs] = createSignal<LogMessage[]>([]);
  const theme = useTheme();

  let parentRef: HTMLDivElement | undefined;

  const rowVirtualizer = createVirtualizer({
    get count() {
      return logs().length;
    },
    getScrollElement: () => parentRef,
    estimateSize: () => 60,
    overscan: 10,
  });

  const updateSizes = debounce(() => {
    rowVirtualizer.getVirtualItems().forEach((item, index) => {
      item.measureElement(parentRef?.children[0].children[index]);
    });
  }, 20);

  const handleScroll = debounce(() => {
    if (!programmaticScroll()) {
      console.log(
        parentRef!.scrollHeight,
        parentRef!.scrollTop + parentRef!.clientHeight
      );
      setAtBottom(
        parentRef!.scrollTop + parentRef!.clientHeight >=
          parentRef!.scrollHeight - 20
      );
    } else {
      setProgrammaticScroll(false);
    }

    setScrollPos(parentRef!.scrollTop);
  }, 10);

  onMount(async () => {
    parentRef?.addEventListener("scroll", handleScroll);
    addEventListener("resize", () => {
      updateSizes();
      setPanelHeight(window.innerHeight - 220);
    });

    setServiceState(await invoke<ServiceInfo>("get_service_info"));

    await listen<ServiceInfo>("service_info", (event) =>
      setServiceState(event.payload)
    );

    await listen<string>("log", (event) => {
      const parsedLog = parse(event.payload).spans.map((s, i) => {
        let cssStr = s.css;
        if (s.color?.dim && !s.color?.name) {
          cssStr = "color:rgba(200,200,200,0.5)";
        }
        return { css: cssStr, text: s.text };
      });

      setLogs((logs) => [...logs, { spans: parsedLog }]);
      if (atBottom()) {
        parentRef?.scrollTo({ top: parentRef.scrollHeight });
      } else {
        parentRef?.scrollTo({ top: scrollPos() });
      }
    });
  });

  const notify = (text: string) =>
    toast(text, {
      style: {
        "background-color": theme.colors.secondaryBackground,
        color: "inherit",
        "border-left": `4px solid ${theme.colors.primary}`,
      },
    });

  const getStartStopText = () => {
    return serviceState().state === "Started" ? "Stop" : "Start";
  };

  const getEnableDisableText = () => {
    return serviceState().autostart ? "Disable" : "Enable";
  };

  const labelWidth = "150px";

  return (
    <>
      <div
        style={{
          display: "flex",
          "padding-bottom": "5px",
          "justify-content": "space-between",
        }}
      >
        <Card minimal>
          <div>
            <AlignedLabel width={labelWidth}>State: </AlignedLabel>
            <div />
          </div>
          <div>
            <AlignedLabel width={labelWidth}>Autostart: </AlignedLabel>
            <div />
          </div>
          <div>
            <AlignedLabel width={labelWidth}>Health: </AlignedLabel>
            <div />
          </div>
          <div>
            <AlignedLabel width={labelWidth}>Exit Code: </AlignedLabel>
            <div />
          </div>
          <div>
            <AlignedLabel width={labelWidth}>PID: </AlignedLabel>
            <div />
          </div>
        </Card>
        <Group>
          <Button
            onClick={() => {
              invoke("toggle_start_stop");
              notify(
                `Service ${
                  serviceState().state === "Started" ? "stopped" : "started"
                }`
              );
            }}
          >
            {getStartStopText()}
          </Button>
          <Button
            onClick={async () => {
              await invoke("restart");
              notify("Service restarted");
            }}
          >
            Restart
          </Button>
          <Button
            onClick={async () => {
              await invoke("toggle_enable_disable");
              notify(
                `Service ${serviceState().autostart ? "disabled" : "enabled"}`
              );
            }}
          >
            {getEnableDisableText()}
          </Button>
        </Group>
      </div>

      <Tabs default="logs">
        <TabBar>
          <Tab value="logs">Logs</Tab>
        </TabBar>

        <TabContent value="logs">
          <Card
            ref={parentRef}
            style={{
              height: `${panelHeight()}px`,
              "font-family": "monospace",
              overflow: "auto",
            }}
          >
            <div
              style={{
                height: `${rowVirtualizer.getTotalSize()}px`,
                width: "100%",
                position: "relative",
              }}
            >
              {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                const row = logs()[virtualRow.index];
                return (
                  <div
                    ref={(el) => onMount(() => virtualRow.measureElement(el))}
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      transform: `translateY(${virtualRow.start}px)`,
                    }}
                  >
                    {row?.spans.map((s) => (
                      <span style={s.css}>{s.text}</span>
                    ))}
                  </div>
                );
              })}
            </div>
          </Card>
        </TabContent>
      </Tabs>
    </>
  );
}

export default App;
