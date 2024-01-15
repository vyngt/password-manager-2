"use client";

import { SchemeColumn, useTree } from "@/components/ColorScheme";
import { ColorScheme } from "@/components/ColorScheme/define";
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
  DeleteButton,
  FieldColumn,
  Row,
  Table,
} from "@/components/UI/TreeView/Table";
import { TreeData } from "@/components/UI/TreeView/define";
import { invoke } from "@tauri-apps/api/tauri";
import { useRouter } from "next/navigation";
import { useCallback, useEffect } from "react";

export default function ColorSchemeTree() {
  const [state, dispatch] = useTree();

  const router = useRouter();

  const browseRecord = useCallback(
    (id: number) => {
      router.push(`/main/color-scheme/form#id=${id}`);
    },
    [router],
  );

  const fetchColorSchemes = useCallback(
    async ({ q = "", page = 1 }: { q?: string; page?: number }) => {
      const result = await invoke<TreeData<ColorScheme>>(
        "fetch_color_schemes",
        {
          page,
          term: q,
        },
      );
      return result;
    },
    [],
  );

  const handleQuery = useCallback(
    async (q: string) => {
      const result = await fetchColorSchemes({ q });
      dispatch({ type: "filter", payload: { data: result, searchTerm: q } });
    },
    [fetchColorSchemes, dispatch],
  );

  const handlePagination = async (page: number) => {
    const data = await fetchColorSchemes({ page, q: state.lastTerm });
    dispatch({ type: "paginate", payload: { data, nextPage: page } });
  };

  const refresh = useCallback(async () => {
    const result = await fetchColorSchemes({});
    dispatch({ type: "load", payload: { data: result } });
  }, [fetchColorSchemes, dispatch]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <TreeView
      header={
        <Header
          action={
            <>
              <FormAction href="/main/color-scheme/form#id=0" />
              <RefreshAction handle={refresh} />
            </>
          }
          filter={<SearchFilter handleQuery={handleQuery} />}
          paginator={
            <Paginator
              currentPage={state.currentPage}
              itemCount={state.itemCount}
              handlePage={handlePagination}
            />
          }
        />
      }
    >
      <Table name="color-scheme" headers={["Name", "Scheme", ""]}>
        {state.items.map((e) => (
          <Row key={e.id}>
            <FieldColumn data={e.name} onClick={() => browseRecord(e.id)} />
            <Column>
              <SchemeColumn data={e} />
            </Column>
            <Column>
              <DeleteButton
                id={e.id}
                refetchId={"fetch_color_schemes"}
                deleteId={"delete_color_scheme"}
                state={state}
                dispatch={dispatch}
              />
            </Column>
          </Row>
        ))}
      </Table>
    </TreeView>
  );
}
