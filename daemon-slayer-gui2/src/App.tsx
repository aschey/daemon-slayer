import { createEffect, createSignal, JSX, onMount } from 'solid-js';
import logo from './assets/logo.svg';
import { invoke } from '@tauri-apps/api/tauri';
import './App.css';
import { Group } from './Group';
import { Button } from './Button';
import { Tab, TabBar, TabContent, Tabs } from './Tabs';
import { Card } from './Card';
import { listen } from '@tauri-apps/api/event';
import { parse } from 'ansicolor';
import { css, useTheme } from 'solid-styled-components';
import toast from 'solid-toast';
import { createVirtualizer } from '@tanstack/solid-virtual';

type LogMessage = {
  spans: { css: string; text: string }[];
};

function App() {
  const [serviceState, setServiceState] = createSignal('');
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

  onMount(async () => {
    setServiceState(await invoke<string>('get_service_state'));

    await listen<{ serviceState: string }>('service_state', (event) =>
      setServiceState(event.payload.serviceState)
    );

    await listen<string>('log', (event) => {
      const parsedLog = parse(event.payload).spans.map((s, i) => {
        let cssStr = s.css;
        if (s.color?.dim && !s.color?.name) {
          cssStr = 'color:rgba(125,125,125,0.5)';
        }
        return { css: cssStr, text: s.text };
      });
      setLogs((logs) => [{ spans: parsedLog }, ...logs]);
      console.log(logs().length);
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      rowVirtualizer.measureElementCache = {};
      rowVirtualizer.measure();
    });
  });

  const notify = (text: string) =>
    toast(text, {
      style: {
        'background-color': theme.colors.secondaryBackground,
        color: 'inherit',
        'border-left': `4px solid ${theme.colors.primary}`,
      },
    });

  const getButtonText = () => {
    return serviceState() === 'started' ? 'Stop' : 'Start';
  };

  return (
    <>
      <Group style={{ 'padding-bottom': '5px' }}>
        <Button
          onClick={() => {
            invoke('toggle');
            notify(
              `Service ${serviceState() === 'started' ? 'stopped' : 'started'}`
            );
          }}
        >
          {getButtonText()}
        </Button>
        <Button
          onClick={() => {
            invoke('restart');
            notify('Service restarted');
          }}
        >
          Restart
        </Button>
      </Group>
      <Tabs default='logs'>
        <TabBar>
          <Tab value='logs'>Logs</Tab>
        </TabBar>

        <TabContent value='logs'>
          <Card style={{ height: '480px', 'font-family': 'monospace' }}>
            <div
              ref={parentRef}
              style={{
                height: `480px`,
                overflow: 'auto',
              }}
            >
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  width: '100%',
                  position: 'relative',
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const row = logs()[virtualRow.index];
                  return (
                    <div
                      ref={(el) => onMount(() => virtualRow.measureElement(el))}
                      style={{
                        position: 'absolute',
                        top: 0,
                        left: 0,
                        width: '100%',
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
            </div>
          </Card>
        </TabContent>
      </Tabs>
    </>
  );
}

export default App;
