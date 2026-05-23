export type FileBaseUploadResult = {
  fileId: string;
  url: string;
  duplicate: boolean;
};

export type FileBaseUploadSession = {
  uploadUrl: string;
  token: string;
  expiresAt: string;
};
