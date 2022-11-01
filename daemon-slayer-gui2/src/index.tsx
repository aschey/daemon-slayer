/* @refresh reload */
import { render } from 'solid-js/web';

import './style.css';
import App from './App';
import {
  createGlobalStyles,
  ThemeProp,
  ThemeProvider,
} from 'solid-styled-components';
import { Toaster } from 'solid-toast';

const theme = {
  colors: {
    primary: 'rgb(24,100,171)',
    primaryBackground: '#2f2f2f',
    secondaryBackground: '#3f3f3f',
    text: '#ffffff',
  },
};

const GlobalStyles = () => {
  const Styles = createGlobalStyles`
    :root {
      background: ${(props: ThemeProp) =>
        props.theme?.colors.primaryBackground};
      color: ${(props: ThemeProp) => props.theme?.colors.text};
      font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
      font-size: 16px;
      line-height: 24px;
      font-weight: 400;
      font-synthesis: none;
      text-rendering: optimizeLegibility;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
      -webkit-text-size-adjust: 100%;
    }
  `;
  return <Styles />;
};

render(
  () => (
    <ThemeProvider theme={theme}>
      <GlobalStyles />
      <Toaster />
      <App />
    </ThemeProvider>
  ),
  document.getElementById('root') as HTMLElement
);
