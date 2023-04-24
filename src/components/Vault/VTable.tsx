import type { GItem } from "@/models";
export const VRow = ({ item }: { item: GItem }) => {
  return (
    <tr className="border-b transition duration-300 ease-in-out hover:bg-neutral-100 dark:border-neutral-500 dark:hover:bg-neutral-600">
      <td className="whitespace-nowrap px-6 py-4">{item.name}</td>
      <td className="whitespace-nowrap px-6 py-4">{item.url}</td>
      <td className="whitespace-nowrap px-6 py-4">{item.username}</td>
      <td className="whitespace-nowrap px-6 py-4">Copy | Edit | Delete</td>
    </tr>
  );
};

export const VTable = ({ items }: { items: GItem[] }) => {
  return (
    <div className="grow">
      <div className="overflow-x-auto sm:-mx-6 lg:-mx-8">
        <div className="inline-block min-w-full py-2 sm:px-6 lg:px-8">
          <div className="overflow-hidden">
            <table className="min-w-full text-left text-sm font-light">
              <thead className="border-b font-medium dark:border-neutral-500">
                <tr>
                  <th scope="col" className="px-6 py-4">
                    Name
                  </th>
                  <th scope="col" className="px-6 py-4">
                    URL
                  </th>
                  <th scope="col" className="px-6 py-4">
                    Username
                  </th>
                  <th scope="col" className="px-6 py-4">
                    Operator
                  </th>
                </tr>
              </thead>
              <tbody className="object-scale-down ">
                {items.map((item) => (
                  <VRow key={item.id} item={item} />
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
};
