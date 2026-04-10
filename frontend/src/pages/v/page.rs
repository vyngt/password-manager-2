use leptos::prelude::*;

use ui::components::Button;
use ui::components::table::{Table, TableBody, TableCell, TableColumn, TableHeader, TableRow};
use ui::primitives::tokens::Variant;

const HEADERS: [&'static str; 4] = ["name", "url", "identity", "action"];

#[component]
pub fn VPage() -> impl IntoView {
    view! {
        <div class="h-full">
            <Table>
                <TableHeader class="bg-secondary/20">
                    <TableRow>
                        {HEADERS.iter().map(|n| view! {
                            <TableColumn class="p-2 hover:bg-secondary/30 transition-colors uppercase">{n.to_string()}</TableColumn>
                        }).collect_view()}
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <TableRow>
                        <TableCell class="font-jetbrains-mono">"Hello world"</TableCell>
                        <TableCell class="font-jetbrains-mono">"Hello world"</TableCell>
                        <TableCell class="font-jetbrains-mono">"Hello world"</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                </TableBody>
            </Table>
        </div>
    }
}
