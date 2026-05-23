export type { FileBaseUploadResult, FileBaseUploadSession } from "@filebase/shared";

export type FileBaseClientOptions = {
  signEndpoint: string;
};

export class FileBaseClient {
  constructor(readonly options: FileBaseClientOptions) {}
}
