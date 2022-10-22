import { AppProps } from 'next/app';
import { ColorScheme, Button } from '@mantine/core';

const Index = (props: AppProps & { colorScheme: ColorScheme }) => {
  return <Button>Settings</Button>;
};

export default Index;
