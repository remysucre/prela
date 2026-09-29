WITH UserStats AS (
    SELECT
        U.Id AS UserId,
        U.DisplayName,
        U.Reputation,
        U.CreationDate,
        COUNT(DISTINCT P.Id) AS PostCount,
        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
        SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
        SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore
    FROM
        Users U
    LEFT JOIN
        Posts P ON U.Id = P.OwnerUserId
    GROUP BY
        U.Id, U.DisplayName, U.Reputation, U.CreationDate
),
PostActivity AS (
    SELECT
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.LastActivityDate,
        PH.PostHistoryTypeId,
        PH.CreationDate AS HistoryDate,
        PH.UserDisplayName,
        PH.UserId,
        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
    FROM
        Posts P
    LEFT JOIN
        PostHistory PH ON P.Id = PH.PostId
    LEFT JOIN
        Comments C ON P.Id = C.PostId
    GROUP BY
        P.Id, P.Title, P.CreationDate, P.LastActivityDate, PH.PostHistoryTypeId, PH.CreationDate, PH.UserDisplayName, PH.UserId
)
SELECT
    US.UserId,
    US.DisplayName,
    US.Reputation,
    US.PostCount,
    US.Questions,
    US.Answers,
    US.TotalViews,
    US.TotalScore,
    PA.PostId,
    PA.Title,
    PA.CreationDate AS PostCreationDate,
    PA.LastActivityDate AS PostLastActivityDate,
    PA.PostHistoryTypeId,
    PA.HistoryDate,
    PA.UserDisplayName AS HistoryUserDisplayName,
    PA.CommentCount
FROM
    UserStats US
JOIN
    PostActivity PA ON PA.UserId = US.UserId
ORDER BY
    US.Reputation DESC, PA.LastActivityDate DESC, US.UserId, PA.PostId, PA.HistoryDate, PA.PostHistoryTypeId, PA.UserDisplayName
LIMIT 100;