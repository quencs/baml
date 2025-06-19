import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@baml/ui/select';
import { useAtom } from 'jotai';
import { TestPanelViewType, testPanelViewTypeAtom } from './atoms';

export const ViewSelector = () => {
  const [viewType, setViewType] = useAtom(testPanelViewTypeAtom);

  return (
    <Select
      value={viewType}
      onValueChange={(value) => setViewType(value as TestPanelViewType)}
    >
      <SelectTrigger className="h-8 w-[120px]">
        <SelectValue placeholder="Select view" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="tabular">Table View</SelectItem>
        <SelectItem value="card">
          Detailed View
        </SelectItem>
        {/* <SelectItem value="client_graph">Client Graph</SelectItem> */}
        {/* <SelectItem value="simple">
          Card View (Simple)
        </SelectItem> */}
      </SelectContent>
    </Select>
  );
};
