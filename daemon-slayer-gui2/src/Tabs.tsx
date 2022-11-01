import {
  Accessor,
  createContext,
  createSignal,
  JSX,
  Setter,
  useContext,
} from 'solid-js';
import { css, DefaultTheme, styled, useTheme } from 'solid-styled-components';

const TabContext = createContext<TabData>();

interface TabData {
  selected: Accessor<string>;
  setSelected: Setter<string>;
  theme: DefaultTheme;
}

export const Tabs = (props: { children: JSX.Element; default: string }) => {
  const [selected, setSelected] = createSignal(props.default);
  const theme = useTheme();

  return (
    <TabContext.Provider value={{ selected, setSelected, theme }}>
      {props.children}
    </TabContext.Provider>
  );
};

const useTabs = () => useContext(TabContext);

export const Tab = (props: { value: string; children: JSX.Element }) => {
  const tabData = useTabs();
  return (
    <button
      class={css`
        cursor: pointer;
        border: none;
        border-bottom: ${props.value === tabData?.selected()
          ? `2px solid ${tabData?.theme.colors.primary}`
          : '0'};

        appearance: none;
        background: none;
        color: inherit;
      `}
      onClick={() => tabData?.setSelected(props.value)}
    >
      {props.children}
    </button>
  );
};

export const TabContent = (props: { value: string; children: JSX.Element }) => {
  const tabData = useTabs();
  return (
    <div
      class={css`
        display: ${props.value === tabData?.selected() ? 'block' : 'none'};
        padding: 6px 12px;
        border-top: none;
      `}
    >
      {props.children}
    </div>
  );
};

export const TabBar = styled.div`
  overflow: hidden;
`;
