/** @jsxImportSource @emotion/react */

import { AppProps } from "next/app";
import { ColorScheme, Button, Table, Tabs, AppShell } from "@mantine/core";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { parse } from "ansicolor";
import { css } from "@emotion/react";
import { EmotionJSX } from "@emotion/react/types/jsx-namespace";
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  Row,
  useReactTable,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";

type LogMessage = {
  spans: EmotionJSX.Element[];
};

const Index = (props: AppProps & { colorScheme: ColorScheme }) => {
  const [serviceState, setServiceState] = useState("");
  const [logs, setLogs] = useState<LogMessage[]>([]);

  const tableContainerRef = useRef<HTMLDivElement>(undefined);

  const rowVirtualizer = useVirtualizer({
    count: logs.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => 60,
  });

  const columnHelper = createColumnHelper<LogMessage>();
  const columns = [
    columnHelper.accessor("spans", { cell: (info) => info.getValue() }),
  ];

  useEffect(() => {
    invoke<string>("get_service_state").then(setServiceState);

    const unlistenServiceState = listen<{ serviceState: string }>(
      "service_state",
      (event) => setServiceState(event.payload.serviceState)
    );

    const unlistenLogs = listen<string>("log", (event) => {
      const parsedLog = parse(event.payload).spans.map((s, i) => (
        <span
          key={i}
          css={css`
            ${s.css}
          `}
        >
          {s.text}
        </span>
      ));

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
      <Button onClick={() => invoke("toggle")}>{getButtonText()}</Button>
      <Button onClick={() => invoke("restart")}>Restart</Button>
      <Tabs defaultValue="logs">
        <Tabs.Tab value="logs">Logs</Tabs.Tab>
        <Tabs.Panel value="logs">
          <div
            ref={tableContainerRef}
            style={{
              height: "490px",
              overflow: "auto",
            }}
          >
            <div
              style={{
                height: `${rowVirtualizer.getTotalSize()}px`,
                width: "100%",
                position: "relative",
                fontFamily: "monospace",
              }}
            >
              <tbody>
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const row = logs[virtualRow.index];
                  return (
                    <div
                      key={virtualRow.index}
                      ref={virtualRow.measureElement}
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
              </tbody>
            </div>
          </div>
        </Tabs.Panel>
      </Tabs>
    </AppShell>
  );
};

export default Index;
