import { JSX } from "solid-js/jsx-runtime";
import { styled } from "solid-styled-components";

export const AlignedLabel = (props: {
  width: string;
  children: JSX.Element;
}) => {
  const Label = styled.label`
    display: inline-block;
    width: ${props.width};
    text-align: right;
    white-space: pre;
    font-weight: bold;
  `;

  return <Label>{props.children}</Label>;
};
