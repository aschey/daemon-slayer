/** @jsxImportSource @emotion/react */

import { AppProps } from "next/app";
import { ColorScheme, Button, Table } from "@mantine/core";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { parse } from "ansicolor";
import { css } from "@emotion/react";
import { EmotionJSX } from "@emotion/react/types/jsx-namespace";
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";

type LogMessage = {
  spans: EmotionJSX.Element[];
};

const Index = (props: AppProps & { colorScheme: ColorScheme }) => {
  const [serviceState, setServiceState] = useState("");
  const [logs, setLogs] = useState<LogMessage[]>([]);

  const columnHelper = createColumnHelper<LogMessage>();
  const columns = [
    columnHelper.accessor("spans", { cell: (info) => info.getValue() }),
  ];

  const table = useReactTable({
    data: logs,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

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
    <>
      <Button onClick={() => invoke("toggle")}>{getButtonText()}</Button>
      <Button onClick={() => invoke("restart")}>Restart</Button>
      <Table>
        <tbody>
          {table.getRowModel().rows.map((row) => (
            <tr key={row.id}>
              {row.getVisibleCells().map((cell) => (
                <td key={cell.id}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </Table>
    </>
  );
};

export default Index;
