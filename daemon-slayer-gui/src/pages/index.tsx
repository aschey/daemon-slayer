/** @jsxImportSource @emotion/react */

import { AppProps } from "next/app";
import {
  ColorScheme,
  Button,
  Table,
  Tabs,
  AppShell,
  Group,
  Card,
} from "@mantine/core";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { parse } from "ansicolor";
import { css } from "@emotion/react";
import { EmotionJSX } from "@emotion/react/types/jsx-namespace";
import { useVirtual } from "react-virtual";
import { showNotification } from "@mantine/notifications";

type LogMessage = {
  spans: EmotionJSX.Element[];
};

const Index = (props: AppProps & { colorScheme: ColorScheme }) => {
  const [serviceState, setServiceState] = useState("");
  const [logs, setLogs] = useState<LogMessage[]>([]);

  const parentRef = useRef<HTMLDivElement>(undefined);

  const rowVirtualizer = useVirtual({
    size: logs.length,
    overscan: 10,
    parentRef,
  });

  useEffect(() => {
    invoke<string>("get_service_state").then(setServiceState);

    const unlistenServiceState = listen<{ serviceState: string }>(
      "service_state",
      (event) => setServiceState(event.payload.serviceState)
    );

    const unlistenLogs = listen<string>("log", (event) => {
      const parsedLog = parse(event.payload).spans.map((s, i) => {
        let cssStr = s.css;
        if (s.color?.dim && !s.color?.name) {
          cssStr = "color:rgba(125,125,125,0.5)";
        }
        return (
          <span
            key={i}
            css={css`
              ${cssStr}
            `}
          >
            {s.text}
          </span>
        );
      });

      setLogs((logs) => [{ spans: parsedLog }, ...logs]);
    });
    return () => {
      unlistenServiceState.then((unlisten) => unlisten());
      unlistenLogs.then((unlisten) => unlisten());
    };
  }, []);

  const getButtonText = () => {
    return serviceState === "started" ? "Stop" : "Start";
  };
  return (
    <AppShell>
      <Group style={{ paddingBottom: "5px" }}>
        <Button
          style={{ fontFamily: "system-ui" }}
          onClick={() => {
            invoke("toggle");
            showNotification({
              message: `Service ${
                serviceState === "started" ? "stopped" : "started"
              }`,
            });
          }}
        >
          {getButtonText()}
        </Button>
        <Button
          style={{ fontFamily: "system-ui" }}
          onClick={() => {
            invoke("restart");
            showNotification({ message: "Service restarted" });
          }}
        >
          Restart
        </Button>
      </Group>

      <Tabs defaultValue="logs">
        <Tabs.Tab value="logs">Logs</Tabs.Tab>
        <Tabs.Panel value="logs">
          <Card
            ref={parentRef}
            style={{
              marginTop: "10px",
              height: "480px",
              overflow: "auto",
              position: "relative",
            }}
          >
            <div
              style={{
                height: `${rowVirtualizer.totalSize}px`,
                position: "relative",
                fontFamily: "monospace",
              }}
            >
              {rowVirtualizer.virtualItems.map((virtualRow) => {
                const row = logs[virtualRow.index];
                return (
                  <div
                    key={virtualRow.index}
                    ref={virtualRow.measureRef}
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      transform: `translateY(${virtualRow.start}px)`,
                    }}
                  >
                    {row.spans}
                  </div>
                );
              })}
            </div>
          </Card>
        </Tabs.Panel>
      </Tabs>
    </AppShell>
  );
};

export default Index;
