import { AppProps } from "next/app";
import { ColorScheme, Button } from "@mantine/core";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

const Index = (props: AppProps & { colorScheme: ColorScheme }) => {
  const [serviceState, setServiceState] = useState("");
  useEffect(() => {
    invoke<string>("get_service_state").then(setServiceState);
    const unlistenPromise = listen<{ serviceState: string }>(
      "service_state",
      (event) => setServiceState(event.payload.serviceState)
    );
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const getButtonText = () => {
    return serviceState === "started" ? "Stop" : "Start";
  };
  return (
    <>
      <Button onClick={() => invoke("toggle")}>{getButtonText()}</Button>
      <Button onClick={() => invoke("restart")}>Restart</Button>
    </>
  );
};

export default Index;
