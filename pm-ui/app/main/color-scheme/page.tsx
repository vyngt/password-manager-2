"use client";

import { useTree } from "@/components/ColorScheme";
import { TreeView } from "@/components/UI";
import {
  FormAction,
  Header,
  Paginator,
  RefreshAction,
  SearchFilter,
} from "@/components/UI/TreeView/Header";
import {
  Column,
  FieldColumn,
  Row,
  Table,
} from "@/components/UI/TreeView/Table";

export default function ColorSchemeTree() {
  const [state, dispatch] = useTree();

  return (
    <TreeView
      header={
        <Header
          action={
            <>
              <FormAction href="/main/color-scheme/form#id=0" />
              <RefreshAction handle={() => {}} />
            </>
          }
          filter={<SearchFilter handleQuery={() => {}} />}
          paginator={
            <Paginator
              currentPage={state.currentPage}
              itemCount={state.itemCount}
              handlePage={() => {}}
            />
          }
        />
      }
    >
      <Table name="color-scheme" headers={["Name", "Scheme", ""]}>
        {state.items.map((e) => (
          <Row key={e.id}>
            <FieldColumn data={e.name} onClick={() => {}} />
            <Column>Scheme</Column>
            <Column>Action</Column>
          </Row>
        ))}
      </Table>
    </TreeView>
  );
}
