// import original module declarations
import 'solid-styled-components';

// and extend them!
declare module 'solid-styled-components' {
  export interface DefaultTheme {
    colors: {
      primary: string;
      primaryBackground: string;
      secondaryBackground: string;
      text: string;
    };
  }
}
