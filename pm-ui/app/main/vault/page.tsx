"use client";

import { useCallback, useEffect } from "react";
import { useRouter } from "next/navigation";
import { useTree } from "@/components/Vault";
import { TreeView } from "@/components/UI/TreeView";
import {
  Header,
  FormAction,
  Paginator,
  RefreshAction,
  SearchFilter,
} from "@/components/UI/TreeView/Header";
import {
  Table,
  Column,
  FieldColumn,
  Row,
} from "@/components/UI/TreeView/Table";

import type { TreeData } from "@/components/UI/TreeView/define";
import type { Item } from "@/components/Vault/define";

import { invoke } from "@tauri-apps/api/tauri";
import { CopyKeyButton } from "@/components/Vault/CopyKeyButton";
import { DeleteButton } from "@/components/Vault/DeleteButton";

export default function VaultView() {
  const [state, dispatch] = useTree();
  const router = useRouter();

  const browseRecord = useCallback(
    (id: number) => {
      router.push(`/main/vault/form#id=${id}`);
    },
    [router],
  );

  const fetchItems = useCallback(
    async ({ q = "", page = 1 }: { q?: string; page?: number }) => {
      const result = await invoke<TreeData<Item>>("fetch_items", {
        page,
        term: q,
      });
      return result;
    },
    [],
  );

  const handleQuery = useCallback(
    async (q: string) => {
      const result = await fetchItems({ q });
      dispatch({ type: "filter", payload: { data: result, searchTerm: q } });
    },
    [fetchItems, dispatch],
  );

  const handlePagination = async (page: number) => {
    const data = await invoke<TreeData<Item>>("fetch_items", {
      page: page,
      term: state.lastTerm,
    });

    dispatch({ type: "paginate", payload: { data, nextPage: page } });
  };

  const refresh = useCallback(async () => {
    const result = await fetchItems({});
    dispatch({ type: "load", payload: { data: result } });
  }, [fetchItems, dispatch]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <TreeView
      header={
        <Header
          action={
            <>
              <FormAction href="/main/vault/form#id=0" />
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
      <Table headers={["Name", "URL", "Username", ""]}>
        {state.items.map((e) => (
          <Row key={e.id}>
            <FieldColumn data={e.name} onClick={() => browseRecord(e.id)} />
            <FieldColumn data={e.url} onClick={() => browseRecord(e.id)} />
            <FieldColumn data={e.username} onClick={() => browseRecord(e.id)} />
            <Column>
              <CopyKeyButton id={e.id} />
              <DeleteButton id={e.id} />
            </Column>
          </Row>
        ))}
      </Table>
    </TreeView>
  );
}
