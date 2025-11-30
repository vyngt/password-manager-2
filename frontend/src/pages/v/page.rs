use leptos::prelude::*;

use ui::components::Button;
use ui::components::table::{Table, TableBody, TableCell, TableColumn, TableHeader, TableRow};
use ui::primitives::tokens::Variant;

#[component]
pub fn VPage() -> impl IntoView {
    view! {
        <div class="h-full">
            <Table>
                <TableHeader>
                    <TableColumn>Username</TableColumn>
                    <TableColumn>Action</TableColumn>
                </TableHeader>
                <TableBody>
                    <TableRow>
                        <TableCell class="font-jetbrains-mono">"Hello world"</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell class="font-yomogi">"Cooked cooked"</TableCell>
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
