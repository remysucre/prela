WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Body,
        U.DisplayName AS Author,
        COUNT(C.id) AS CommentCount,
        COUNT(A.id) AS AnswerCount,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.CreationDate DESC) AS RN
    FROM 
        Posts P
        LEFT JOIN Comments C ON P.Id = C.PostId
        LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
        LEFT JOIN Votes V ON P.Id = V.PostId
        LEFT JOIN Users U ON P.OwnerUserId = U.Id
    WHERE 
        P.PostTypeId = 1 
    GROUP BY 
        P.Id, P.Title, P.CreationDate, P.Body, U.DisplayName
),

HighScorePosts AS (
    SELECT 
        RP.PostId,
        RP.Title,
        RP.CreationDate,
        RP.Body,
        RP.Author,
        RP.CommentCount,
        RP.AnswerCount,
        RP.UpVotes,
        RP.DownVotes,
        (RP.UpVotes - RP.DownVotes) AS Score,
        RANK() OVER (ORDER BY (RP.UpVotes - RP.DownVotes) DESC) AS PostRank
    FROM 
        RankedPosts RP
)

SELECT 
    HSP.PostId,
    HSP.Title,
    HSP.CreationDate,
    HSP.Body,
    HSP.Author,
    HSP.CommentCount,
    HSP.AnswerCount,
    HSP.Score,
    TagArray
FROM 
    HighScorePosts HSP
JOIN (
    SELECT 
        P.Id AS PostId,
        STRING_AGG(T.TagName, ', ' ORDER BY T.TagName) AS TagArray
    FROM 
        Posts P
        CROSS JOIN LATERAL UNNEST(STRING_TO_ARRAY(SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2), '><')) AS T(TagName)
    GROUP BY 
        P.Id
) AS Tags ON HSP.PostId = Tags.PostId
WHERE 
    HSP.PostRank <= 10 
ORDER BY 
    HSP.Score DESC;