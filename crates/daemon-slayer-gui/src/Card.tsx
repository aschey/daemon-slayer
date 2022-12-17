import { JSX } from "solid-js/jsx-runtime";
import { styled } from "solid-styled-components";

interface CardProps
  extends JSX.DOMAttributes<HTMLDivElement>,
    JSX.HTMLAttributes<HTMLDivElement> {
  minimal?: boolean;
}

export const Card = (props: CardProps) => {
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
  return <Container {...props}>{props.children}</Container>;
};
