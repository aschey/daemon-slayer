import { styled } from 'solid-styled-components';

export const Button = styled.button`
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  background-color: ${(props) => props.theme?.colors.primary};
  color: inherit;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
  cursor: pointer;
  outline: none;
  &:hover {
    border-color: rgb(50, 50, 0);
  }
`;
