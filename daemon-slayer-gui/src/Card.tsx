import { JSX } from "solid-js/jsx-runtime";
import { styled } from "solid-styled-components";

export const Card = (props: {
  minimal?: boolean;
  style?: JSX.CSSProperties;
  children: JSX.Element;
}) => {
  const Container = styled.div`
    background: ${(styledProps) =>
      props.minimal
        ? undefined
        : styledProps.theme?.colors.secondaryBackground};
    border-radius: 8px;
    padding: 10px;
    border: ${(styledProps) =>
      props.minimal
        ? `2px solid ${styledProps.theme?.colors.primary}`
        : undefined};
  `;
  return <Container style={props.style}>{props.children}</Container>;
};
