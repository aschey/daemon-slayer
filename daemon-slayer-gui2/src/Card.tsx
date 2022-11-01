import { styled } from 'solid-styled-components';

export const Card = styled.div`
  background: ${(props) => props.theme?.colors.secondaryBackground};
  border-radius: 8px;
`;
